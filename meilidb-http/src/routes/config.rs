use log::*;
use std::error;
use std::sync::Arc;

use meilidb::database::{Database, Config};
use rocket::{Route, State};
use rocket_contrib::json::{Json, JsonValue};

use crate::guard::AdminGuard;

#[get("/")]
fn retrieve(db: State<Arc<Database>>, guard: AdminGuard) -> Result<String, Box<error::Error>> {
    info!("retrieve - Start handler - index: {:?}", guard.index);
    let config = db.get_config(&guard.index)?;

    Ok(serde_json::to_string(&config)?)
}

#[post("/", format = "application/json", data = "<body>")]
fn update(body: Json<Config>, db: State<Arc<Database>>, guard: AdminGuard) -> Result<JsonValue, Box<error::Error>> {
    info!("update - Start handler - index: {:?}", guard.index);
    let body = body.into_inner();

    let mut saved_config = match db.get_config(&guard.index) {
        Ok(config) => config,
        Err(_) => {
            let _ = db.update_config(&guard.index, body);
            return Ok(json!({
                "success": true,
            }));
        }
    };

    saved_config.update_with(body);

    if let Some(stop_words) = saved_config.stop_words.clone() {
        if stop_words.is_empty() {
            saved_config.stop_words = None;
        }
    };
    if let Some(ranking_order) = saved_config.ranking_order.clone() {
        if ranking_order.is_empty() {
            saved_config.ranking_order = None;
        }
    };
    if let Some(distinct_field) = saved_config.distinct_field.clone() {
        if distinct_field.is_empty() {
            saved_config.distinct_field = None;
        }
    };
    if let Some(ranking_rules) = saved_config.ranking_rules.clone() {
        if ranking_rules.is_empty() {
            saved_config.ranking_rules = None;
        }
    };
    if let Some(access_token) = saved_config.access_token.clone() {
        if access_token.read_key.is_empty() || access_token.write_key.is_empty() || access_token.admin_key.is_empty() {
            saved_config.access_token = None;
        }
    };

    let _ = db.update_config(&guard.index, saved_config);

    Ok(json!({
        "success": true,
    }))
}

// route "/config"
pub fn routes() -> Vec<Route> {
    routes![retrieve, update]
}
