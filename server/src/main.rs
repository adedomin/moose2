/* Copyright (C) 2024  Anthony DeDominic
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

// use moosedb::MooseDb;
use crate::{
    config::SubComm,
    model::moose::moose_bulk_transform,
    task::{dump_moose_task, shutdown_task, web_task},
};

use db::MooseDB;
use tokio::sync::broadcast::{self, Receiver, Sender};

pub mod config;
pub mod db;
pub mod middleware;
pub mod model;
pub mod render;
pub mod shared_data;
pub mod task;
pub mod templates;
pub mod web_handlers;

#[cfg(unix)]
fn main() {
    user_main()
}

#[cfg(windows)]
fn main() {
    // if error, the software is not running as a service.
    if svc_main().is_err() {
        user_main();
    }
}

fn user_main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();
    let (stopchan_tx, stopchan_rx) = broadcast::channel(1);
    if let Err(e) = real_main(stopchan_tx, stopchan_rx, false) {
        match e {
            config::ArgsError::Usage(usage) => {
                if !usage.is_empty() {
                    log::error!("{usage}")
                };
                eprintln!("{}", config::USAGE);
            }
            e => log::error!("{e}"),
        }
    }
}

#[cfg(windows)]
fn svc_main() -> Result<(), &'static str> {
    use windows_services::{Command, Service};

    let (stopchan_tx, _) = broadcast::channel(1);
    let mut thread = None;
    let mut logger_set_up = false;
    Service::new().can_stop().run(move |service, msg| {
        if !logger_set_up {
            if let Ok(logfile) = config::get_service_logfile() {
                env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
                    .target(env_logger::Target::Pipe(logfile))
                    .init();
            }
            logger_set_up = true;
        }
        match msg {
            Command::Start => {
                log::debug!("Service Starting...");
                if thread.is_none() {
                    let st_tx = stopchan_tx.clone();
                    let st_rx = st_tx.subscribe();
                    // SAFETY: `service` parameter should be a valid reference for as long as the windows service is running.
                    //          We always join this handle inside the service handler's stop routine.
                    //
                    //          Note that a std::thread::Scope still fails because `service` escapes its closure when the JoinHandle is stored
                    //          in `thread`.
                    //
                    // The example https://github.com/microsoft/windows-rs/blob/master/crates/samples/services/thread/src/main.rs#L20-L21
                    // does something similar since `pool.submit` accepts a closure with the same lifetime as &Service<'_>
                    thread = Some(unsafe {
                        std::thread::Builder::new()
                            .spawn_unchecked(move || {
                                if let Err(e) = real_main(st_tx, st_rx, true) {
                                    log::error!("{e}");
                                    // only set status on abnormal termination.
                                    service.set_state(windows_services::State::Stopped);
                                }
                            })
                            .map_err(|e| {
                                log::error!("Failed to spawn thread: {e}");
                                e
                            })
                            .unwrap()
                    });
                }
            }
            Command::Stop => {
                log::warn!("Windows asked us to stop; stopping...");
                if let Some(jh) = thread.take() {
                    _ = stopchan_tx.send(());
                    _ = jh.join();
                }
            }
            // unsupported command
            _ => (),
        }
    })
}

fn real_main(
    stopchan_tx: Sender<()>,
    stopchan_rx: Receiver<()>,
    win_service: bool,
) -> Result<(), config::ArgsError> {
    let (subcmd, rc) = config::parse_args()?;
    if let SubComm::Convert(io) = subcmd {
        // We do not need the runtime or database for this
        log::info!("Converting moose-legacy format to moose2 format.");
        let (moose_in, moose_out) = match io {
            Some((i, o)) => (Some(i), o),
            None => (None, None),
        };
        moose_bulk_transform(moose_in, moose_out);
        return Ok(());
    }

    #[cfg(not(feature = "multi-thread"))]
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap();
    #[cfg(feature = "multi-thread")]
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap();
    rt.block_on(async {
        log::info!("Connecting to database: {:?}", rc.get_moose_path());
        let db = db::utils::open_db(&rc).await;

        if let SubComm::Import(dup_behavior, moose_in) = subcmd {
            log::info!("Importing moose. Shutting down after importing.");
            db.bulk_import(moose_in, dup_behavior).await?;
            return Ok(());
        }

        // make sure our DB actually works and we can open it (no permission issues for instance).
        db.check_pool().await?;

        let moose_dump_file = rc.get_moose_dump();
        let dbx = db.clone();
        let dump_task = dump_moose_task(moose_dump_file, rc.get_moose_path(), dbx, stopchan_rx);

        let stopchan_rx = stopchan_tx.subscribe();
        let web_task = web_task(rc, db, stopchan_rx);

        let shutdown_task = shutdown_task(stopchan_tx, win_service);

        let _ = tokio::try_join!(shutdown_task, web_task, dump_task)
            .expect("All tasks to start/shutdown successfully.");
        Ok(())
    })
}
