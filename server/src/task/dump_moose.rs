use std::{
    io::{BufWriter, Write},
    path::PathBuf,
    time::Duration,
};

use tokio::{sync::broadcast::Receiver, time};

use crate::db::{query::DUMP_MOOSE, Pool};
use crate::model::{self};

#[derive(thiserror::Error, Debug)]
pub enum DumpTaskError {
    #[error("Pool Connection Interaction Error: {0}")]
    PoolInteractError(#[from] deadpool_sqlite::InteractError),
    #[error("Sqlite3 Error: {0}")]
    Sqlite3(#[from] rusqlite::Error),
    #[error("Could not open dump file: {0}")]
    DumpFile(#[from] std::io::Error),
    #[error("Deserialization Error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub async fn dump_moose(
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
                println!("INFO: [DUMP] Timer Triggered, dumping Moose to json file.");
                let con = db.get().await.unwrap();
                let md = moose_dump.clone();
                con.interact(move |con| -> Result<(), DumpTaskError> {
                    let file = std::fs::File::create(md)?;
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
                    println!("INTO: [DUMP] Done dumping moose.");
                    Ok(())
                })
                .await??;
            }
        }
    }
}
