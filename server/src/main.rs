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
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .unwrap();
    rt.block_on(async {
        let (subcmd, rc) = config::parse_args();
        if let Some(sub) = subcmd {
            match sub {
                SubCommand::Import { input } => {
                    moose_bulk_import(input, &rc).await;
                    return;
                }
                SubCommand::Convert { input, output } => {
                    moose_bulk_transform(input, output);
                    return;
                }
            }
        }

        println!(
            "INFO: [MAIN] Connecting to database: {:?}",
            rc.get_moose_path()
        );
        let db = db::open_db(&rc).await;

        let moose_dump_file = rc.get_moose_dump();
        let dbx = db.clone();
        let (stopchan_tx, rx1) = broadcast::channel(1);
        let dump_task = dump_moose_task(moose_dump_file, dbx, rx1);

        let rx2 = stopchan_tx.subscribe();
        let web_task = web_task(rc, db, rx2);

        let shutdown_task = shutdown_task(stopchan_tx);

        let _ = tokio::try_join!(shutdown_task, web_task, dump_task)
            .expect("All tasks to start/shutdown successfully.");
    });
}
