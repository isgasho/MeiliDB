use log::*;
use std::error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rocket::{Route, State};
use rocket_contrib::json::Json;
use serde_derive::{Deserialize, Serialize};

use crate::guard::SuperAdminGuard;

pub struct Health(Arc<AtomicBool>);

impl Health {
    pub fn new() -> Health {
        Health(Arc::new(AtomicBool::new(true)))
    }

    pub fn get(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn set(&self, val: bool) {
        self.0.store(val, Ordering::Relaxed);
    }
}

#[derive(Serialize, Deserialize)]
struct HealthChange {
    health: bool
}

#[get("/")]
fn health(
    health: State<Health>,
    guard: SuperAdminGuard
) -> Result<(), Box<error::Error>> {
    let _ = guard;
    if health.get() {
        return Ok(());
    }
    Err("Unhealthy".into())
}

#[put("/", format = "application/json", data = "<body>")]
fn change_health(
    body: Json<HealthChange>,
    health: State<Health>,
    guard: SuperAdminGuard
) -> Result<(), Box<error::Error>>
{
    let _ = guard;
    info!("change_health - Start handler");
    Ok(health.set(body.health))
}

// route "/health"
pub fn routes() -> Vec<Route> {
    routes![health, change_health]
}
