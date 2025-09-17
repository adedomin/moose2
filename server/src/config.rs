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

use crate::{
    db::{BulkModeDupe, sqlite3_impl::Sqlite3Error},
    shared_data::EXAMPLE_CONFIG,
};
use bcrypt_pbkdf::bcrypt_pbkdf;
use serde::Deserialize;
use std::{
    env, fs,
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
};

#[cfg(unix)]
mod env_vars {
    pub mod config {
        pub const BASE: &str = "CONFIGURATION_DIRECTORY";
        pub const USER: &str = "XDG_CONFIG_HOME";
        pub const FALLBACK: &str = "/etc";
    }
    pub mod data {
        pub const BASE: &str = "STATE_DIRECTORY";
        pub const USER: &str = "XDG_DATA_HOME";
        pub const FALLBACK: &str = "/var/lib";
    }
}
#[cfg(windows)]
mod env_vars {
    pub mod config {
        pub const BASE: &str = "MOOSE2_HOME";
        pub const USER: &str = "AppData";
        pub const FALLBACK: &str = r"C:\ProgramData";
    }
    pub mod data {
        pub use super::config::{BASE, FALLBACK, USER};
    }
}

use env_vars::{config, data};

const PBKDF_SALT: &[u8] = br####";o'"#|`=8kZhT:DWK\x4#<:&C.#Rzdd@"####;
const PBKDF_ROUNDS: u32 = 8u32;

#[derive(Debug, thiserror::Error)]
pub enum ArgsError {
    #[error("Config file not found; created example one at location: {0:?} ")]
    NoConfig(PathBuf),
    #[error("Failed to deserialize config: {0}")]
    DeserConfig(#[from] serde_json::Error),
    #[error("Unknown IO error: {0}")]
    IoErr(#[from] io::Error),
    #[error("Invalid cookie secret: {0}")]
    Bcrypt(#[from] bcrypt_pbkdf::Error),
    #[error("usage err: {0}")]
    Usage(String),
    #[error("Cannot get database connection: {0}")]
    DbConn(#[from] Sqlite3Error),
}

#[derive(Deserialize, Clone)]
pub struct GitHubOauth2 {
    pub id: String,
    pub secret: String,
    pub redirect: Option<String>,
}

#[derive(Clone)]
pub struct Secret(pub [u8; 64]);

impl Default for Secret {
    fn default() -> Self {
        Secret(std::array::from_fn(|_| rand::random()))
    }
}

#[derive(Default, Deserialize, Clone)]
pub struct RunConfig {
    moose_path: Option<PathBuf>,
    moose_dump: Option<PathBuf>,
    listen: Option<String>,
    cookie_secret: Option<String>,
    pub github_oauth2: Option<GitHubOauth2>,
    #[serde(skip)]
    pub cookie_key: Secret,
}

impl RunConfig {
    pub fn get_moose_path(&self) -> PathBuf {
        if let Some(path) = &self.moose_path {
            path.clone()
        } else {
            find_systemd_or_xdg_path(data::BASE, data::USER, data::FALLBACK, "moose2.db")
        }
    }

    pub fn get_moose_dump(&self) -> PathBuf {
        if let Some(path) = &self.moose_dump {
            path.clone()
        } else {
            find_systemd_or_xdg_path(data::BASE, data::USER, data::FALLBACK, "moose2.json")
        }
    }

    pub fn get_bind_addr(&self) -> String {
        self.listen
            .as_ref()
            .map(|l| l.to_owned())
            .unwrap_or_else(|| "[::1]:5921".to_owned())
    }
}

pub fn create_parent_dirs<T: AsRef<Path>>(path: T) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)
    } else {
        Ok(())
    }
}

/// It's assumed the package name, moose2 is the "Above Path" in the XDG and fallback case.
fn find_systemd_or_xdg_path(systemd: &str, xdg: &str, fallback: &str, dest: &str) -> PathBuf {
    let mut base = std::env::var_os(systemd)
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os(xdg).map(|p| {
                let mut p = PathBuf::from(p);
                p.push(env!("CARGO_PKG_NAME"));
                p
            })
        })
        .unwrap_or_else(|| {
            let mut p = PathBuf::from(fallback);
            p.push(env!("CARGO_PKG_NAME"));
            p
        });
    base.push(dest);
    base
}

#[cfg(windows)]
pub fn get_service_logfile() -> io::Result<Box<std::io::LineWriter<std::fs::File>>> {
    let fpath = find_systemd_or_xdg_path(data::BASE, data::USER, data::FALLBACK, "moose2.log");
    Ok(Box::new(std::io::LineWriter::new(std::fs::File::create(
        fpath,
    )?)))
}

pub fn open_or_write_default<T>(config_path: T) -> Result<RunConfig, ArgsError>
where
    T: std::fmt::Debug + AsRef<Path>,
{
    match std::fs::File::open(&config_path) {
        Ok(file) => {
            let file = BufReader::new(file);
            Ok(serde_json::from_reader(file)?)
        }
        Err(_) => {
            log::error!("Configuration file not found or could not be opened at: {config_path:?}",);
            log::info!("Creating configuration file... ");
            create_parent_dirs(&config_path)?;
            let mut file = std::fs::File::create(&config_path)?;
            file.write_all(EXAMPLE_CONFIG)?;
            log::info!("Edit the file {config_path:?} and restart the application.");
            Err(ArgsError::NoConfig(config_path.as_ref().to_path_buf()))
        }
    }
}

struct Comm {
    config: Option<PathBuf>,
    listen: Option<String>,
    dupe: BulkModeDupe,
    subcmd: SubComm,
}
pub enum SubComm {
    Run,
    Import(BulkModeDupe, Option<PathBuf>),
    Convert(Option<(PathBuf, Option<PathBuf>)>),
}

impl Default for Comm {
    fn default() -> Self {
        Self {
            config: None,
            listen: None,
            dupe: BulkModeDupe::Fail,
            subcmd: SubComm::Run,
        }
    }
}

pub const USAGE: &str = r###"usage: moose2 [OPTIONS] [SUBCOMMAND]

Options:
    -c | --config=c  Configuration file to read from; default: $CONFIGURATION_DIRECTORY/config.json
                                                              $XDG_CONFIG_HOME/moose2/config.json
    -l | --listen=l  server listen address argument; overrides configuration file.
    -i | --ignore    for import subcommand: ignore existing duplicate moose (by name).
    -u | --update    for import subcommand: update existing duplicate moose (by name).

Subcommand:
    import  [input]      Import moose from [input] json file.
    convert [from] [to]  Convert moose json dump to modern moose2 format.
"###;

fn parse_argv() -> Result<Comm, ArgsError> {
    enum F {
        Config,
        Listen,
    }
    let (comm, flag) = std::env::args()
        .skip(1)
        .fold(vec![], |mut args, arg| {
            if (arg.starts_with("-c")
                || arg.starts_with("--config")
                || arg.starts_with("-l")
                || arg.starts_with("--listen"))
                && let Some((f, v)) = arg.split_once('=')
            {
                args.push(f.to_owned());
                args.push(v.to_owned());
            } else {
                args.push(arg);
            }
            args
        })
        .into_iter()
        .try_fold((Comm::default(), None), |(mut comm, flag_slot), arg| {
            if let Some(flag) = flag_slot {
                match flag {
                    F::Config => comm.config = Some(arg.into()),
                    F::Listen => comm.listen = Some(arg.to_owned()),
                }
                return Ok((comm, None));
            };
            let mut flag_slot = None;
            match arg.as_str() {
                "-c" | "--config" => flag_slot = Some(F::Config),
                "-l" | "--listen" => flag_slot = Some(F::Listen),
                "-i" | "--ignore" => comm.dupe = BulkModeDupe::Ignore,
                "-u" | "--update" => comm.dupe = BulkModeDupe::Update,
                "-h" | "--help" => return Err(ArgsError::Usage("".to_owned())),
                arg if arg.starts_with('-') => {
                    return Err(ArgsError::Usage(format!("Unknown Flag {arg}.")));
                }
                arg => match (comm.subcmd, arg) {
                    (SubComm::Run, "import") => {
                        comm.subcmd = SubComm::Import(BulkModeDupe::Fail, None)
                    }
                    (SubComm::Run, "convert") => comm.subcmd = SubComm::Convert(None),
                    (SubComm::Run, anything) => {
                        return Err(ArgsError::Usage(format!("Invalid subcommand {anything}.")));
                    }
                    (SubComm::Import(d, None), file) => {
                        comm.subcmd = SubComm::Import(d, Some(file.into()));
                    }
                    (SubComm::Import(_, Some(_)), _) => {
                        return Err(ArgsError::Usage("Too many arguments to import.".to_owned()));
                    }
                    (SubComm::Convert(None), file) => {
                        comm.subcmd = SubComm::Convert(Some((file.into(), None)));
                    }
                    (SubComm::Convert(Some((input, None))), output) => {
                        comm.subcmd = SubComm::Convert(Some((input, Some(output.into()))));
                    }
                    (SubComm::Convert(Some((_, Some(_)))), _) => {
                        return Err(ArgsError::Usage(
                            "Too many files given to convert.".to_owned(),
                        ));
                    }
                },
            }
            Ok((comm, flag_slot))
        })?;
    if flag.is_some() {
        Err(ArgsError::Usage("No value given for flag.".to_owned()))
    } else {
        Ok(comm)
    }
}

pub fn parse_args() -> Result<(SubComm, RunConfig), ArgsError> {
    let args = parse_argv()?;
    let sub = match args.subcmd {
        SubComm::Import(_, input) => SubComm::Import(args.dupe, input),
        sc => sc,
    };

    let config_file_path = match args.config {
        Some(c) => c,
        None => {
            find_systemd_or_xdg_path(config::BASE, config::USER, config::FALLBACK, "config.json")
        }
    };
    let mut conf = open_or_write_default(config_file_path)?;
    if let Some(listen) = args.listen {
        // seems better to have explicit command line override configuration.
        conf.listen = Some(listen);
    }
    // Secret::default() auto initializes with random bytes.
    if let Some(user_secret) = &conf.cookie_secret {
        bcrypt_pbkdf(
            user_secret,
            PBKDF_SALT,
            PBKDF_ROUNDS,
            &mut conf.cookie_key.0,
        )?;
    }
    Ok((sub, conf))
}
