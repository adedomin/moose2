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

use tokio::{sync::broadcast::error::SendError, task::JoinHandle};
use tokio_util::sync::CancellationToken;

#[cfg(unix)]
pub fn shutdown_task(
    stop_token: CancellationToken,
    _win_service: bool,
) -> JoinHandle<Result<(), SendError<()>>> {
    use tokio::signal::{ctrl_c, unix};

    log::info!("Setting up shutdown listener.");
    tokio::spawn(async move {
        let mut sigterm = unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
        tokio::select! {
            _ = ctrl_c() => {
                log::warn!("SIGINT: shutting down.");
            }
            _ = sigterm.recv() => {
                log::warn!("SIGTERM: shutting down.");
            }
            _ = stop_token.cancelled() => {
                log::warn!("CANCELED: shutting down.");
            }
        }
        stop_token.cancel();
        Ok(())
    })
}

#[cfg(windows)]
pub fn shutdown_task(
    stop_token: CancellationToken,
    win_service: bool,
) -> JoinHandle<Result<(), SendError<()>>> {
    use tokio::signal::windows;

    // the service manager will signal shutdown; just exit early.
    if win_service {
        log::info!("Running as Windows Service; not running shutdown listener.");
        tokio::spawn(async move { Ok(()) })
    } else {
        // using console or IIS HttpPlatformHandler (?)
        log::info!("Running as Windows Console app; setting up Console Ctrl handlers.");
        tokio::spawn(async move {
            let mut ctrl_break = windows::ctrl_break().unwrap();
            let mut ctrl_c = windows::ctrl_c().unwrap();
            let mut ctrl_close = windows::ctrl_close().unwrap();
            let mut ctrl_logoff = windows::ctrl_logoff().unwrap();
            let mut ctrl_shutdown = windows::ctrl_shutdown().unwrap();
            tokio::select! {
                _ = ctrl_break.recv() => {
                    log::warn!("Ctrl-BREAK: shutting down.");
                }
                _ = ctrl_c.recv() => {
                    log::warn!("Ctrl-C: shutting down.");
                }
                _ = ctrl_close.recv() => {
                    log::warn!("Ctrl-Close: shutting down.");
                }
                _ = ctrl_logoff.recv() => {
                    log::warn!("Ctrl-Logoff: shutting down.");
                }
                _ = ctrl_shutdown.recv() => {
                    log::warn!("Ctrl-Shutdown: shutting down.");
                }
                _ = stop_token.cancelled() => {
                    log::warn!("CANCELED: shutting down.");
                }
            }
            stop_token.cancel();
            Ok(())
        })
    }
}
