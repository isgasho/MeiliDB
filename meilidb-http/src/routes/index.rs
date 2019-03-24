use log::*;
use std::error;
use std::sync::Arc;

use meilidb::database::{Database, SchemaBuilder, Config};
use rocket::{Route, State};
use rocket_contrib::json::Json;

use crate::guard::{SuperAdminGuard, AdminGuard};

#[derive(Serialize, Deserialize)]
pub struct IndexCreation {
    pub index_name: String,
    pub schema: SchemaBuilder,
    pub config: Option<Config>,
}

#[get("/")]
fn list(
    db: State<Arc<Database>>,
    guard: SuperAdminGuard
) -> Result<String, Box<error::Error>> {
    info!("list - Start handler - index: {:?}", guard.index);
    let list = db.list_indexes();

    Ok(serde_json::to_string(&list)?)
}

// Create an index. Only accessible with an Administrator Key
#[post("/", format = "application/json", data = "<body>")]
fn create(
    body: Json<IndexCreation>,
    db: State<Arc<Database>>,
    guard: SuperAdminGuard,
) -> Result<(), Box<error::Error>> {
    let _ = guard;
    let content = body.into_inner();

    info!("create - Start handler - index: {:?}", content.index_name);

    if let Err(err) = db.create_index(&content.index_name, &content.schema.build()) {
        warn!("Index: {}; {}", content.index_name, err);
    };

    Ok(())
}

#[delete("/")]
fn delete(
    db: State<Arc<Database>>,
    guard: AdminGuard,
) -> Result<(), Box<error::Error>> {
    info!("delete - Start handler - index: {}", guard.index);

    if let Err(err) = db.delete_index(&guard.index) {
        warn!("Index: {}; {}", guard.index, err);
    }

    Ok(())
}

// route "/indices"
pub fn routes() -> Vec<Route> {
    routes![list, create, delete]
}
