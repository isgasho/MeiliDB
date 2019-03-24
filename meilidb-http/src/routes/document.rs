use log::*;
use std::error;
use std::sync::Arc;
use std::collections::HashSet;
use std::collections::HashMap;

use rocket::{Route, State};
use meilidb::database::Database;
use serde_derive::{Deserialize, Serialize};
use rocket_contrib::json::{Json, JsonValue};

use crate::guard::WriteGuard;

// A single document is a JSON. But with the restriction to only have key and values as String
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Document(pub HashMap<String, String>);

// A list of document is a Vec of Object Document so a Vec of JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Documents(pub Vec<Document>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentsUpdate {
    pub insert: Option<Documents>,
    pub delete: Option<Documents>,
}


#[patch("/", format = "application/json", data = "<body>")]
fn update(
    body: Json<DocumentsUpdate>,
    db: State<Arc<Database>>,
    guard: WriteGuard,
) -> Result<JsonValue, Box<error::Error>> {
    info!("update - Start handler - index: {}", guard.index);

    let update = body.into_inner();

    let mut builder = db.start_update(&guard.index)?;
    let stop_words = &db
        .view(&guard.index)?
        .config()
        .stop_words
        .clone()
        .unwrap_or(HashSet::new());

    if let Some(documents) = update.insert {
        for document in documents.0 {
            if let Err(err) = builder.update_document(document.0, stop_words) {
                warn!("Impossible to insert document; {}", err);
            }
        }
    }

    if let Some(documents) = update.delete {
        for document in documents.0 {
            if let Err(err) = builder.remove_document(document.0) {
                warn!("Impossible to delete document; {}", err);
            }
        }
    }

    let _ = db.commit_update(builder);

    Ok(json!({}))
}

#[put("/", format = "application/json", data = "<body>")]
fn insert(
    body: Json<Documents>,
    db: State<Arc<Database>>,
    guard: WriteGuard,
) -> Result<JsonValue, Box<error::Error>> {
    info!("insert - Start handler - index: {}", guard.index);

    let mut builder = db.start_update(&guard.index)?;

    let stop_words = &db
        .view(&guard.index)?
        .config()
        .stop_words
        .clone()
        .unwrap_or(HashSet::new());

    for document in body.into_inner().0 {
        if let Err(err) = builder.update_document(document.0, stop_words) {
            warn!("Impossible to insert document; {}", err);
        }
    }

    let _ = db.commit_update(builder);

    Ok(json!({}))
}

#[delete("/", format = "application/json", data = "<body>")]
fn delete(
    body: Json<Documents>,
    db: State<Arc<Database>>,
    guard: WriteGuard,
) -> Result<JsonValue, Box<error::Error>> {
    info!("delete - Start handler - index: {}", guard.index);

    let mut builder = db.start_update(&guard.index)?;

    for document in body.into_inner().0 {
        if let Err(err) = builder.remove_document(document.0) {
            warn!("Impossible to delete document; {}", err);
        }
    }

    let _ = db.commit_update(builder);

    Ok(json!({}))
}

// route "/documents"
pub fn routes() -> Vec<Route> {
    routes![update, insert, delete]
}
