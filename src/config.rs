use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug, Default)]
#[clap(version)]
pub struct Args {
    /// Path to the moose storage
    #[clap(short, long)]
    moose: Option<String>,
    /// Listen string, if starts with /, assumes uds
    #[clap(short, long)]
    listen: Option<String>,
    #[clap(subcommand)]
    pub command: SubArg,
}

#[derive(Subcommand, Debug)]
pub enum SubArg {
    Import {
        #[clap(short, long)]
        file: Option<PathBuf>,
        #[clap(short, long)]
        output: Option<PathBuf>,
    },
    Run,
}

impl Default for SubArg {
    fn default() -> Self {
        SubArg::Run
    }
}

impl Args {
    pub fn get_moose_path(self) -> PathBuf {
        match &self.moose {
            Some(path) => PathBuf::from(path),
            None => std::env::vars()
                .find(|(key, _value)| key == "XDG_DATA_HOME" || key == "STATE_DIRECTORY")
                .and_then(|(key, val)| match key.as_str() {
                    "XDG_DATA_HOME" => {
                        let mut r = PathBuf::from(val);
                        r.push("moose2/moose.json");
                        Some(r)
                    }
                    "STATE_DIRECTORY" => {
                        let mut r = PathBuf::from(val);
                        r.push("moose.json");
                        Some(r)
                    }
                    _ => unreachable!(),
                })
                .unwrap_or_else(|| PathBuf::from("./moose.json")),
        }
    }

    pub fn get_bind_addr(self) -> String {
        self.listen.unwrap_or_else(|| "[::1]:5921".to_owned())
    }
}
