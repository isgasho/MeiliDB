#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate envconfig_derive;

mod option;
mod server;
mod guard;
mod routes;

use log::*;
use std::error::Error;
use std::sync::Arc;

use meilidb::database::Database;

use option::Opt;
use server::start_server;
use guard::SuperAdminToken;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn main() -> Result<(), Box<Error>> {
    let opt = Opt::new();

    let db_path = opt.database_path;
    let db = Arc::new(Database::open(db_path.clone()).unwrap());

    info!("Start listening HTTP");
    let token = SuperAdminToken(opt.token);

    start_server(db, token, opt.port).launch();

    Ok(())
}
