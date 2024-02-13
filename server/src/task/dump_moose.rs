use std::{
    fs::File,
    io::{BufWriter, IntoInnerError, Write},
    path::PathBuf,
    time::Duration,
};

use rand::Rng;
use tokio::{sync::broadcast::Receiver, task::JoinHandle, time};

use crate::db::{query::DUMP_MOOSE, Connection, Pool};
use crate::model::{self};

#[derive(thiserror::Error, Debug)]
pub enum DumpTaskError {
    #[error("Pool Connection Interaction Error: {0}")]
    PoolInteractError(#[from] deadpool_sqlite::InteractError),
    #[error("Sqlite3 Error: {0}")]
    Sqlite3(#[from] rusqlite::Error),
    #[error(
        "Could not open tmp file, sync() it to disk or rename tmp moose to moose_dump path: {0}"
    )]
    StdIO(#[from] std::io::Error),
    #[error("Deserialization Error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Failed to recover file handle: {0}")]
    IntoInner(#[from] IntoInnerError<BufWriter<File>>),
    #[error("Moose dump path is either \"/\" or an empty string, \"\".")]
    StrangeMooseDumpPath(),
}

async fn dump_moose_real(con: Connection, moose_dump: PathBuf) -> Result<(), DumpTaskError> {
    con.interact(move |con| -> Result<(), DumpTaskError> {
        // parent only fails when totally rooted.
        let tdir = match moose_dump.parent() {
            Some(p) => p,
            None => return Err(DumpTaskError::StrangeMooseDumpPath()),
        };
        let r: u64 = rand::thread_rng().gen();
        let tdir = tdir.join(format!(".moose.json.{:x}", r));

        let file = File::create(&tdir)?;
        let mut bufw = BufWriter::new(file);
        let mut start = true;

        let mut q = con.prepare_cached(DUMP_MOOSE)?;
        let mut w = q.query([])?;
        while let Ok(Some(row)) = w.next() {
            if start {
                bufw.write(b"[")?;
                start = false;
            } else {
                bufw.write(b",")?;
            }
            let moose = model::moose::Moose {
                name: row.get(0)?,
                image: row.get(1)?,
                dimensions: row.get(2)?,
                created: row.get(3)?,
                author: row.get(4)?,
                upvotes: row.get(5)?,
            };
            let moose = serde_json::to_vec(&moose)?;
            bufw.write(&moose)?;
        }
        bufw.write(b"]")?;
        let inner = bufw.into_inner()?;
        inner.sync_data()?;
        drop(inner);

        std::fs::rename(tdir, moose_dump)?;

        println!("INFO: [DUMP] Done dumping moose.");
        Ok(())
    })
    .await?
}

async fn dump_moose(
    moose_dump: PathBuf,
    db: Pool,
    mut stop_broadcast: Receiver<bool>,
) -> Result<(), DumpTaskError> {
    let mut interval = time::interval(Duration::from_secs(3600));

    loop {
        tokio::select! {
            _ = stop_broadcast.recv() => {
                return Ok(());
            },
            _ = interval.tick() => {
                println!("INFO: [DUMP] Timer Triggered, dumping Moose to json file: {moose_dump:?}");
                let con = db.get().await.unwrap();
                let md = moose_dump.clone();
                dump_moose_real(con, md).await?;
            }
        }
    }
}

pub fn dump_moose_task(
    moose_dump: PathBuf,
    db: Pool,
    stop_broadcast: Receiver<bool>,
) -> JoinHandle<Result<(), DumpTaskError>> {
    println!("INFO: [DUMP] Setting up Auto-dumps of database.");
    tokio::spawn(async move {
        let e = dump_moose(moose_dump, db, stop_broadcast).await;
        println!("WARN: [DUMP] Task has shut down: {:?}", e);
        e
    })
}
