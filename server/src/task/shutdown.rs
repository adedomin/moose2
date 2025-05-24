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

use crate::config::SubComm;
use tokio::{
    sync::broadcast::{Sender, error::SendError},
    task::JoinHandle,
};

#[cfg(unix)]
pub fn shutdown_task(
    shutdown_channel: Sender<()>,
    _subcmd: SubComm,
) -> JoinHandle<Result<(), SendError<()>>> {
    tokio::spawn(async move {
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("WARN: [SHUTDOWN] SIGINT: shutting down.");
            }
            _ = sigterm.recv() => {
                println!("WARN: [SHUTDOWN] SIGTERM: shutting down.");
            }
        }
        shutdown_channel.send(())?;
        Ok(())
    })
}

#[cfg(windows)]
pub fn shutdown_task(
    shutdown_channel: Sender<()>,
    subcmd: SubComm,
) -> JoinHandle<Result<(), SendError<()>>> {
    // the service manager will signal shutdown; just exit early.
    if let SubComm::Svc = subcmd {
        tokio::spawn(async move { Ok(()) })
    } else {
        tokio::spawn(async move {
            _ = tokio::signal::ctrl_c().await;
            println!("WARN: [SHUTDOWN] SIGINT: shutting down.");
            shutdown_channel.send(())?;
            Ok(())
        })
    }
}
