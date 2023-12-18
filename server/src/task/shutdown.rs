use tokio::{
    signal::unix::SignalKind,
    sync::broadcast::{error::SendError, Sender},
    task::JoinHandle,
};

pub fn shutdown_task(shutdown_channel: Sender<bool>) -> JoinHandle<Result<(), SendError<bool>>> {
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("WARN: [SHUTDOWN] SIGINT: shutting down.");
            }
            _ = sigterm.recv() => {
                println!("WARN: [SHUTDOWN] SIGTERM: shutting down.");
            }
        }
        shutdown_channel.send(true)?;
        Ok(())
    })
}
