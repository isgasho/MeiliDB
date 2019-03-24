use log::*;
use std::collections::HashMap;
use std::error;
use std::sync::Arc;
use std::iter::FromIterator;

use meilidb::database::Database;
use meilidb_search;
use rocket::request::Form;
use rocket::{Route, State};
use rocket_contrib::json::Json;
use serde_json::Value;

use crate::guard::ReadGuard;

#[derive(FromForm)]
struct Query {
    q: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

#[derive(Serialize, Deserialize)]
struct Body {
    cropped_fields: Option<HashMap<String, usize>>, // field_name : size
    retrievable_fields: Option<Vec<String>>, //All
    searchable_fields: Option<Vec<String>>, //All
}

#[post("/?<query..>", format = "application/json", data = "<body>")]
fn search(
    query: Form<Query>,
    body: Json<Body>,
    db: State<Arc<Database>>,
    guard: ReadGuard,
) -> Result<String, Box<error::Error>> {
    info!("search - Start handler - index: {:?}", guard.index);
    let query = query.into_inner();
    let content = body.into_inner();

    let query_search = meilidb_search::Query {
        query: query.q,
        offset: query.offset,
        length: query.limit,
        cropped_fields: content.cropped_fields,
        attributes_to_retrieve: content.retrievable_fields,
        restrict_searchable_attributes: content.searchable_fields,
    };

    let results = meilidb_search::search(guard.index, query_search, &db)?;

    let mut formated_results = Vec::new();
    for result in results {
        let iter = result.hits.clone().into_iter().map(|(k, v)| (k, Value::String(v.to_string())));
        let mut map: HashMap<String, Value> = HashMap::from_iter(iter);
        map.insert("_matches".to_string(), serde_json::to_value(result.matches).unwrap());
        formated_results.push(map);
    }

    Ok(serde_json::to_string(&formated_results)?)
}

// route "/search"
pub fn routes() -> Vec<Route> {
    routes![search]
}
