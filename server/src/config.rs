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

use crate::shared_data::EXAMPLE_CONFIG;
use bcrypt_pbkdf::bcrypt_pbkdf;
use directories::ProjectDirs;
use serde::Deserialize;
use std::{
    fs,
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
    process::exit,
};

const PBKDF_SALT: &[u8] = br####";o'"#|`=8kZhT:DWK\x4#<:&C.#Rzdd@"####;
const PBKDF_ROUNDS: u32 = 8u32;

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
        Secret([0u8; 64])
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
            let pd= ProjectDirs::from("space", "ghetty", "moose2").expect("Could not find default path to put data in. please explicitly define a moose_path in the config.");
            let mut path = PathBuf::from(pd.data_dir());
            path.push("moose2.db");
            path
        }
    }

    pub fn get_moose_dump(&self) -> PathBuf {
        if let Some(path) = &self.moose_dump {
            path.clone()
        } else {
            let pd= ProjectDirs::from("space", "ghetty", "moose2").expect("Could not find default path to put data in. please explicitly define a moose_path in the config.");
            let mut path = PathBuf::from(pd.data_dir());
            path.push("moose2.json");
            path
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

#[derive(clap::Parser, Debug)]
#[command(
    name = "moose2",
    version,
    about = "Nextgen Moose serving and authoring web application."
)]
pub struct Args {
    #[arg(
        short,
        long,
        help = "Explicit configuration file, otherwise relies on paths such as XDG_CONFIG_HOME to find this"
    )]
    pub config: Option<PathBuf>,
    #[arg(
        short,
        long,
        help = "IPv4/6 address or unix:/your/path/here for a unix domain socket"
    )]
    pub listen: Option<String>,
    #[command(subcommand)]
    pub subcommand: Option<SubCommand>,
}

#[derive(clap::Subcommand, Debug)]
pub enum SubCommand {
    #[command(about = "Import a moose dump")]
    Import {
        #[arg(
            short,
            long,
            help = "Ignore existing moose when importing.",
            default_value_t = false
        )]
        merge: bool,
        input: Option<PathBuf>,
    },
    #[command(about = "Convert a moose (js) dump to moose2 dump")]
    Convert {
        input: Option<PathBuf>,
        output: Option<PathBuf>,
    },
}

pub fn find_config() -> PathBuf {
    if let Some(pd) = ProjectDirs::from("space", "ghetty", "moose2") {
        let config_dir = pd.config_dir();
        let mut config = PathBuf::from(config_dir);
        config.push("config.json");
        config
    } else {
        println!(
            "Something is wrong with your environment variables: attempting to use ./config.json"
        );
        PathBuf::from("./config.json")
    }
}

pub fn open_or_write_default<T>(config_path: T) -> Option<RunConfig>
where
    T: std::fmt::Debug + AsRef<Path>,
{
    if let Ok(file) = std::fs::File::open(&config_path) {
        let file = BufReader::new(file);
        Some(serde_json::from_reader(file).unwrap())
    } else {
        println!(
            "Configuration file not found or could not be opened at: {:?}",
            &config_path
        );
        print!("Creating... ");
        create_parent_dirs(&config_path).unwrap();
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(EXAMPLE_CONFIG).unwrap();
        println!("\nEdit the file and restart the application");
        None
    }
}

pub fn parse_args() -> (Option<SubCommand>, RunConfig) {
    let args = <Args as clap::Parser>::parse();
    let config_file_path = args.config.unwrap_or_else(find_config);
    if let Some(mut conf) = open_or_write_default(config_file_path) {
        if let Some(listen) = args.listen {
            // command line listen does not override configuration.
            if conf.listen.is_none() {
                conf.listen = Some(listen);
            }
        }
        match &conf.cookie_secret {
            None => {
                for i in 0..64 {
                    conf.cookie_key.0[i] = rand::random();
                }
            }
            Some(user_secret) => {
                bcrypt_pbkdf(
                    user_secret,
                    PBKDF_SALT,
                    PBKDF_ROUNDS,
                    &mut conf.cookie_key.0,
                )
                .unwrap();
            }
        }
        (args.subcommand, conf)
    } else {
        exit(1)
    }
}
