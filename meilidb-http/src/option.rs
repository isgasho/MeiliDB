use std::path::PathBuf;

use envconfig::Envconfig;
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
pub struct ArgsOpt {
    /// The destination where the database must be created.
    #[structopt(short = "p", long = "database-path")]
    pub database_path: Option<String>,

    /// The address and port to bind the server to.
    #[structopt(long = "port")]
    pub port: Option<u16>,

    #[structopt(short = "t", long = "token")]
    pub token: Option<String>,
}

#[derive(Envconfig, Clone)]
pub struct EnvOpt {
    #[envconfig(from = "MEILI_DATABASE_PATH")]
    pub database_path: Option<String>,

    #[envconfig(from = "MEILI_PORT")]
    pub port: Option<u16>,

    #[envconfig(from = "MEILI_TOKEN")]
    pub token: Option<String>,
}

pub struct Opt {
    pub database_path: PathBuf,
    pub port: u16,
    pub token: Option<String>,
}

impl Opt {
    pub fn new() -> Opt {
        let args = ArgsOpt::from_args();
        let env = EnvOpt::init().unwrap();
        Opt {
            database_path: args
                .database_path
                .or(env.database_path)
                .expect("".into())
                .into(),
            port: args.port.or(env.port).unwrap_or(8000),
            token: args.token.or(env.token),
        }
    }
}
