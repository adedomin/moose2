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

use std::{
    fs::{self},
    io::{self},
    path::PathBuf,
    sync::atomic::AtomicBool,
    time::Duration,
};

use tokio::{sync::broadcast::Receiver, task::JoinHandle, time};

use crate::db::{
    MooseDB,
    sqlite3_impl::{Pool, Sqlite3Error},
};

static NEW_MOOSE_NOTIFY: AtomicBool = AtomicBool::new(false);

/// Let the Moose Dump Task know a new moose has been written.
pub fn notify_new() {
    NEW_MOOSE_NOTIFY.store(true, std::sync::atomic::Ordering::Relaxed)
}

async fn dump_moose(
    moose_dump: PathBuf,
    dbpath: PathBuf,
    db: Pool,
    mut stop_broadcast: Receiver<()>,
) -> Result<(), Sqlite3Error> {
    // check if database was "likely" changed to prevent wastefully dumping every startup.
    let mdc = moose_dump.clone();
    match tokio::task::spawn_blocking(move || -> Result<bool, io::Error> {
        let dump_mtime = fs::metadata(mdc)?.modified()?;
        let db_mtime = fs::metadata(dbpath)?.modified()?;
        Ok(dump_mtime < db_mtime)
    })
    .await
    .unwrap()
    {
        Ok(false) => (),
        // either dumpfile is missing or it is older than the database file.
        _ => notify_new(),
    }

    let mut interval = time::interval(Duration::from_secs(300));

    loop {
        tokio::select! {
            _ = stop_broadcast.recv() => {
                return Ok(());
            },
            _ = interval.tick() => {
                if NEW_MOOSE_NOTIFY.swap(false, std::sync::atomic::Ordering::Relaxed) {
                    println!("INFO: [DUMP] Dumping moose to json file: {moose_dump:?}");
                    db.dump_moose(moose_dump.clone()).await?;
                } else {
                    println!("DEBUG: [DUMP] Timer Triggered, no new moose to dump.");
                }
            }
        }
    }
}

pub fn dump_moose_task(
    moose_dump: PathBuf,
    dbpath: PathBuf,
    db: Pool,
    stop_broadcast: Receiver<()>,
) -> JoinHandle<Result<(), Sqlite3Error>> {
    println!("INFO: [DUMP] Setting up Auto-dumps of database.");
    tokio::spawn(async move {
        let e = dump_moose(moose_dump, dbpath, db, stop_broadcast).await;
        println!("WARN: [DUMP] Task has shut down: {:?}", e);
        e
    })
}
