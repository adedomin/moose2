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
    config::{RunConfig, SubComm},
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
    let (subcmd, rc) = config::parse_args();
    let (stopchan_tx, stopchan_rx) = broadcast::channel(1);
    real_main(subcmd, rc, stopchan_tx, stopchan_rx);
}

#[cfg(windows)]
fn main() {
    let (subcmd, rc) = config::parse_args();
    let (stopchan_tx, stopchan_rx) = broadcast::channel(1);
    if let SubComm::Svc = subcmd {
        let mut thread = None;
        windows_services::Service::new().can_stop().run(move |msg| {
            match msg {
                Command::Start => {
                    thread = std::thread::spawn(move || {
                        real_main(subcmd, rc, stopchan_tx.clone(), stopchan_rx);
                    });
                }
                Command::Stop => {
                    if let Some(thread) = thread {
                        stopchan_tx.send(()).unwrap();
                        _ = thread.join();
                    }
                }
                // unsupported command
                _ => return,
            }
        })
    } else {
        real_main(subcmd, rc, stopchan_tx, stopchan_rx);
    }
}

fn real_main(subcmd: SubComm, rc: RunConfig, stopchan_tx: Sender<()>, stopchan_rx: Receiver<()>) {
    if let SubComm::Convert(io) = subcmd {
        // We do not need the runtime or database for this
        eprintln!("INFO: [MAIN] Converting moose-legacy format to moose2 format.");
        let (moose_in, moose_out) = match io {
            Some((i, o)) => (Some(i), o),
            None => (None, None),
        };
        moose_bulk_transform(moose_in, moose_out);
        return;
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
        println!(
            "INFO: [MAIN] Connecting to database: {:?}",
            rc.get_moose_path()
        );
        let db = db::utils::open_db(&rc).await;

        if let SubComm::Import(dup_behavior, moose_in) = subcmd {
            println!("INFO: [MAIN] Importing moose. Shutting down after importing.");
            db.bulk_import(moose_in, dup_behavior).await.unwrap();
            return;
        }

        let moose_dump_file = rc.get_moose_dump();
        let dbx = db.clone();
        let dump_task = dump_moose_task(moose_dump_file, rc.get_moose_path(), dbx, stopchan_rx);

        let stopchan_rx = stopchan_tx.subscribe();
        let web_task = web_task(rc, db, stopchan_rx);

        let shutdown_task = shutdown_task(stopchan_tx, subcmd);

        let _ = tokio::try_join!(shutdown_task, web_task, dump_task)
            .expect("All tasks to start/shutdown successfully.");
    });
}
