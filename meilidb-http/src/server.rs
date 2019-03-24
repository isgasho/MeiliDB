// use log::*;
use std::sync::Arc;

use meilidb::database::Database;
use rocket::config::Environment;
use rocket::Config;
use rocket::config::Limits;

use crate::routes;
use crate::guard::SuperAdminToken;
use crate::routes::health::Health;

pub fn start_server(
    db: Arc<Database>,
    token: SuperAdminToken,
    port: u16,
) -> rocket::Rocket {

    let mut config = Config::build(Environment::active().unwrap())
        .finalize()
        .unwrap();

    let limits = Limits::new()
        .limit("forms", 64 * 1024 * 1024)
        .limit("json", 64 * 1024 * 1024);

    config.set_port(port);
    config.set_limits(limits);

    let healthy = Health::new();

    rocket::custom(config)
        .manage(db)
        .manage(token)
        .manage(healthy)
        .mount("/documents", routes::document::routes())
        .mount("/indices", routes::index::routes())
        .mount("/config", routes::config::routes())
        .mount("/health", routes::health::routes())
        .mount("/search", routes::search::routes())
}
