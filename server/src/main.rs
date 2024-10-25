// use moosedb::MooseDb;
use crate::{
    config::SubCommand,
    db::moose_bulk_import,
    model::moose::moose_bulk_transform,
    task::{dump_moose_task, shutdown_task, web_task},
};

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
            SubCommand::Import { merge, input } => {
                // We need an async runtime + db open for this.
                is_import = Some((merge, input));
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
        let dump_task = dump_moose_task(moose_dump_file, dbx, stopchan_rx);

        let stopchan_rx = stopchan_tx.subscribe();
        let web_task = web_task(rc, db, stopchan_rx);

        let shutdown_task = shutdown_task(stopchan_tx);

        let _ = tokio::try_join!(shutdown_task, web_task, dump_task)
            .expect("All tasks to start/shutdown successfully.");
    });
}
