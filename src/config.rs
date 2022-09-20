use bcrypt_pbkdf::bcrypt_pbkdf;
use rand::Rng;
use serde::Deserialize;
use std::{
    fs,
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
    process::exit,
};

const PBKDF_SALT: &[u8] = br####";o'"#|`=8kZhT:DWK\x4#<:&C.#Rzdd@"####;
const PBKDF_ROUNDS: u32 = 8u32;

pub static mut RUN_CONFIG: Option<&'static RunConfig> = None;

pub fn get_config() -> &'static RunConfig {
    unsafe { RUN_CONFIG.unwrap() }
}

#[derive(Deserialize)]
pub struct GitHubOauth2 {
    pub id: String,
    pub secret: String,
}

pub struct Secret(pub [u8; 64]);

impl Default for Secret {
    fn default() -> Self {
        Secret([0u8; 64])
    }
}

#[derive(Default, Deserialize)]
pub struct RunConfig {
    moose_path: Option<PathBuf>,
    listen: Option<String>,
    cookie_secret: Option<String>,
    pub github_oauth2: Option<GitHubOauth2>,
    #[serde(skip_serializing, skip_deserializing)]
    pub cookie_key: Secret,
}

impl RunConfig {
    pub fn get_moose_path(&self) -> PathBuf {
        if let Some(path) = &self.moose_path {
            path.clone()
        } else {
            std::env::vars()
                .find(|(key, _value)| key == "XDG_DATA_HOME" || key == "STATE_DIRECTORY")
                .and_then(|(key, val)| match key.as_str() {
                    "XDG_DATA_HOME" => {
                        let mut r = PathBuf::from(val);
                        r.push("moose2/moose2.db");
                        Some(r)
                    }
                    "STATE_DIRECTORY" => {
                        let mut r = PathBuf::from(val);
                        r.push("moose2.db");
                        Some(r)
                    }
                    _ => unreachable!(),
                })
                .unwrap_or_else(|| PathBuf::from("./moose2.db"))
        }
    }

    pub fn get_bind_addr(&self) -> String {
        self.listen
            .as_ref()
            .map(|l| l.to_owned())
            .unwrap_or_else(|| "[::1]:5921".to_owned())
    }
}

pub enum Args {
    Run,
    Import(Option<PathBuf>),
    Convert(Option<PathBuf>, Option<PathBuf>),
}

pub fn create_parent_dirs<T: AsRef<Path>>(path: &T) -> io::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)
    } else {
        Ok(())
    }
}

pub fn parse() -> Args {
    unsafe {
        if RUN_CONFIG.is_some() {
            panic!("Cannot call config::parse() more than once!");
        }
    }

    let mut args = std::env::args_os();
    let argv0 = args.next().unwrap().into_string().unwrap();
    let subcmd = args
        .next()
        .map(|arg| arg.into_string().unwrap())
        .unwrap_or_else(|| "run".to_string());
    let path1 = args.next().map(PathBuf::from);
    let path2 = args.next().map(PathBuf::from);
    match subcmd.as_str() {
        "r" | "run" => {
            let path = if let Some(path) = path1 {
                path
            } else {
                std::env::vars()
                    .find(|(key, _value)| {
                        key == "XDG_CONFIG_HOME" || key == "CONFIGURATION_DIRECTORY"
                    })
                    .and_then(|(key, val)| match key.as_str() {
                        "XDG_CONFIG_HOME" | "CONFIGURATION_DIRECTORY" => {
                            let mut r = PathBuf::from(val);
                            r.push("moose2/moose2.json");
                            Some(r)
                        }
                        _ => unreachable!(),
                    })
                    .unwrap_or_else(|| PathBuf::from("./config.json"))
            };
            if let Ok(file) = std::fs::File::open(&path) {
                let file = BufReader::new(file);
                let run_config: &'static mut RunConfig =
                    Box::leak(serde_json::from_reader(file).unwrap());
                // generate a random cookie secret
                match &run_config.cookie_secret {
                    None => {
                        for i in 0..64 {
                            run_config.cookie_key.0[i] = rand::thread_rng().gen();
                        }
                    }
                    Some(user_secret) => {
                        bcrypt_pbkdf(
                            user_secret,
                            PBKDF_SALT,
                            PBKDF_ROUNDS,
                            &mut run_config.cookie_key.0,
                        )
                        .unwrap();
                    }
                }
                unsafe {
                    RUN_CONFIG = Some(run_config);
                }
                Args::Run
            } else {
                println!(
                    "Configuration file not found or could not be opened at: {:?}",
                    &path
                );
                print!("Creating... ");
                create_parent_dirs(&path).unwrap();
                let mut file = std::fs::File::create(&path).unwrap();
                let _ = file.write(
                    r##"{ "moose_path": "/path/to/store/meese, omit for default: $XDG_DATA_HOME/moose2 | $STATE_DIRECTORY/"
, "listen": "http://[::1]:5921, omit for default. can use unix:/path/to/socket for uds listening."
, "github_oauth2": { "id": "client id, omit whole object to disable auth."
                   , "secret": "client secret"
                   }
}"##.as_bytes(),
                )
                .unwrap();
                println!("\nEdit the file and restart the application");
                exit(1);
            }
        }
        "i" | "import" => Args::Import(path1),
        "c" | "convert" => Args::Convert(path1, path2),
        _ => {
            eprintln!("usage: {} [run [config.json] | import [meese.json] | convert [old_meese.json] [new_meese.json]]", argv0);
            exit(1);
        }
    }
}
