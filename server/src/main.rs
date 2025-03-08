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
    config::SubCommand,
    db::moose_bulk_import,
    model::moose::moose_bulk_transform,
    task::{dump_moose_task, shutdown_task, web_task},
};

use db::BulkModeDupe;
use tokio::sync::broadcast;

pub mod config;
pub mod db;
pub mod model;
pub mod render;
pub mod shared_data;
pub mod task;
pub mod templates;
pub mod web_handlers;

fn main() {
    let mut is_import = None;
    let (subcmd, rc) = config::parse_args();
    if let Some(sub) = subcmd {
        match sub {
            SubCommand::Import {
                merge,
                update,
                input,
            } => {
                // We need an async runtime + db open for this.
                is_import = Some((
                    if update {
                        BulkModeDupe::Update
                    } else if merge {
                        BulkModeDupe::Ignore
                    } else {
                        BulkModeDupe::Fail
                    },
                    input,
                ));
            }
            SubCommand::Convert { input, output } => {
                // We do not need the runtime or database for this
                eprintln!("INFO: [MAIN] Converting moose-legacy format to moose2 format.");
                moose_bulk_transform(input, output);
                return;
            }
        }
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap();
    rt.block_on(async {
        println!(
            "INFO: [MAIN] Connecting to database: {:?}",
            rc.get_moose_path()
        );
        let db = db::open_db(&rc).await;

        if let Some((merge, moose_in)) = is_import {
            println!("INFO: [MAIN] Importing moose. Shutting down after importing.");
            moose_bulk_import(moose_in, merge, db).await;
            return;
        }

        let moose_dump_file = rc.get_moose_dump();
        let dbx = db.clone();
        let (stopchan_tx, stopchan_rx) = broadcast::channel(1);
        let dump_task = dump_moose_task(moose_dump_file, rc.get_moose_path(), dbx, stopchan_rx);

        let stopchan_rx = stopchan_tx.subscribe();
        let web_task = web_task(rc, db, stopchan_rx);

        let shutdown_task = shutdown_task(stopchan_tx);

        let _ = tokio::try_join!(shutdown_task, web_task, dump_task)
            .expect("All tasks to start/shutdown successfully.");
    });
}
