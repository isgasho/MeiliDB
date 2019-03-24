use log::*;
use std::sync::Arc;

use meilidb::database::Database;
use rocket::http::Status;
use rocket::request;
use rocket::request::FromRequest;
use rocket::request::State;
use rocket::Outcome::*;
use rocket::Request;

#[derive(Debug)]
pub enum ApiKeyError {
    Internal,
    IndexNotFound,
    MissingIndexName,
    MissingToken,
    Invalid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuperAdminToken(pub Option<String>);

pub struct ReadGuard {
    pub index: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for ReadGuard {
    type Error = ApiKeyError;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<ReadGuard, Self::Error> {
        debug!("http::guard::ReadGuard - Start Guard");
        let parameter_token: Option<String> =
            request.headers().get_one("token").map(|i| i.to_owned());

        let mut host_composition = match request.headers().get_one("Host") {
            Some(host) => host.split("."),
            None => {
                debug!("http::guard::ReadGuard - not host");
                return Failure((Status::MethodNotAllowed, ApiKeyError::MissingIndexName));
            }
        };

        let index_name = match host_composition.next() {
            Some(index) => index,
            None => {
                debug!("http::guard::ReadGuard - not index name on host");
                return Failure((Status::MethodNotAllowed, ApiKeyError::MissingIndexName));
            }
        };

        let db = match request.guard::<State<Arc<Database>>>() {
            Success(db) => db,
            Failure(_) | Forward(_) => {
                debug!("http::guard::ReadGuard - Impossible to retrieve the database");
                return Failure((Status::MethodNotAllowed, ApiKeyError::Internal));
            }
        };

        let index_config = match db.view(index_name) {
            Ok(view) => view.config().clone(),
            Err(_) => {
                debug!("http::guard::ReadGuard - Impossible to retrieve the index config");
                return Failure((Status::NotFound, ApiKeyError::IndexNotFound));
            }
        };

        let index_token = match index_config.access_token {
            Some(tokens) => tokens.read_key,
            None => {
                debug!("http::guard::ReadGuard - Impossible to retrieve the index token");
                return Success(ReadGuard {
                    index: index_name.to_string(),
                });
            }
        };

        let parameter_token = match parameter_token {
            Some(token) => token,
            None => {
                debug!("http::guard::ReadGuard - No token on header");
                return Failure((Status::MethodNotAllowed, ApiKeyError::MissingToken));
            }
        };

        debug!(
            "http::guard::ReadGuard - param: {} vs index: {}",
            parameter_token, index_token
        );
        if index_token == parameter_token {
            return Success(ReadGuard {
                index: index_name.to_string(),
            });
        } else {
            debug!("http::guard::ReadGuard - Tokens are not equals");
            return Failure((Status::MethodNotAllowed, ApiKeyError::Invalid));
        }
    }
}

pub struct WriteGuard {
    pub index: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for WriteGuard {
    type Error = ApiKeyError;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<WriteGuard, Self::Error> {
        debug!("http::guard::WriteGuard - Start Guard");
        let parameter_token: Option<String> =
            request.headers().get_one("token").map(|i| i.to_owned());

        let mut host_composition = match request.headers().get_one("Host") {
            Some(host) => host.split("."),
            None => {
                debug!("http::guard::WriteGuard - not host");
                return Failure((Status::MethodNotAllowed, ApiKeyError::MissingIndexName));
            }
        };

        let index_name = match host_composition.next() {
            Some(index) => index,
            None => {
                debug!("http::guard::WriteGuard - not index name on host");
                return Failure((Status::MethodNotAllowed, ApiKeyError::MissingIndexName));
            }
        };

        let db = match request.guard::<State<Arc<Database>>>() {
            Success(db) => db,
            Failure(_) | Forward(_) => {
                debug!("http::guard::WriteGuard - Impossible to retrieve the database");
                return Failure((Status::MethodNotAllowed, ApiKeyError::Internal));
            }
        };

        let index_config = match db.view(index_name) {
            Ok(view) => view.config().clone(),
            Err(_) => {
                debug!("http::guard::WriteGuard - Impossible to retrieve the index config");
                return Failure((Status::NotFound, ApiKeyError::IndexNotFound));
            }
        };

        let index_token = match index_config.access_token {
            Some(tokens) => tokens.write_key,
            None => {
                debug!("http::guard::WriteGuard - Impossible to retrieve the index token");
                return Success(WriteGuard {
                    index: index_name.to_string(),
                });
            }
        };

        let parameter_token = match parameter_token {
            Some(token) => token,
            None => {
                debug!("http::guard::WriteGuard - No token on header");
                return Failure((Status::MethodNotAllowed, ApiKeyError::MissingToken));
            }
        };

        debug!(
            "http::guard::WriteGuard - param: {} vs index: {}",
            parameter_token, index_token
        );
        if index_token == parameter_token {
            return Success(WriteGuard {
                index: index_name.to_string(),
            });
        } else {
            debug!("http::guard::WriteGuard - Tokens are not equals");
            return Failure((Status::MethodNotAllowed, ApiKeyError::Invalid));
        }
    }
}

pub struct AdminGuard {
    pub index: String,
}

impl<'a, 'r> FromRequest<'a, 'r> for AdminGuard {
    type Error = ApiKeyError;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<AdminGuard, Self::Error> {
        debug!("http::guard::AdminGuard - Start Guard");
        let parameter_token: Option<String> =
            request.headers().get_one("token").map(|i| i.to_owned());

        let mut host_composition = match request.headers().get_one("Host") {
            Some(host) => host.split("."),
            None => {
                debug!("http::guard::AdminGuard - not host");
                return Failure((Status::MethodNotAllowed, ApiKeyError::MissingIndexName));
            }
        };

        let index_name = match host_composition.next() {
            Some(index) => index,
            None => {
                debug!("http::guard::AdminGuard - not index name on host");
                return Failure((Status::MethodNotAllowed, ApiKeyError::MissingIndexName));
            }
        };

        let db = match request.guard::<State<Arc<Database>>>() {
            Success(db) => db,
            Failure(_) | Forward(_) => {
                debug!("http::guard::AdminGuard - Impossible to retrieve the database");
                return Failure((Status::MethodNotAllowed, ApiKeyError::Internal));
            }
        };

        let index_config = match db.view(index_name) {
            Ok(view) => view.config().clone(),
            Err(_) => {
                debug!("http::guard::AdminGuard - Impossible to retrieve the index config");
                return Failure((Status::NotFound, ApiKeyError::IndexNotFound));
            }
        };

        let index_token = match index_config.access_token {
            Some(tokens) => tokens.admin_key,
            None => {
                debug!("http::guard::AdminGuard - Impossible to retrieve the index token");
                return Success(AdminGuard {
                    index: index_name.to_string(),
                });
            }
        };

        let parameter_token = match parameter_token {
            Some(token) => token,
            None => {
                debug!("http::guard::AdminGuard - No token on header");
                return Failure((Status::MethodNotAllowed, ApiKeyError::MissingToken));
            }
        };

        debug!(
            "http::guard::AdminGuard - param: {} vs index: {}",
            parameter_token, index_token
        );
        if index_token == parameter_token {
            return Success(AdminGuard {
                index: index_name.to_string(),
            });
        } else {
            debug!("http::guard::AdminGuard - Tokens are not equals");
            return Failure((Status::MethodNotAllowed, ApiKeyError::Invalid));
        }
    }
}

pub struct SuperAdminGuard {
    pub index: Option<String>,
}

impl<'a, 'r> FromRequest<'a, 'r> for SuperAdminGuard {
    type Error = ApiKeyError;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<SuperAdminGuard, Self::Error> {
        debug!("http::guard::SuperAdminGuard - Start Guard");
        let super_admin_token = match request.guard::<State<SuperAdminToken>>() {
            Success(cred) => cred.0.clone(),
            Failure(_) | Forward(_) => {
                return Failure((Status::MethodNotAllowed, ApiKeyError::Internal));
            }
        };

        let super_admin_token = match super_admin_token {
            Some(token) => token,
            None => return Failure((Status::MethodNotAllowed, ApiKeyError::Internal)),
        };

        let parameter_token: Option<String> =
            request.headers().get_one("token").map(|i| i.to_owned());

        let mut host_composition = match request.headers().get_one("Host") {
            Some(host) => host.split("."),
            None => {
                debug!("http::guard::WriteGuard - not host");
                return Failure((Status::MethodNotAllowed, ApiKeyError::MissingIndexName));
            }
        };

        let index_name = host_composition.next().map(|i| i.to_owned());

        let parameter_token = match parameter_token {
            Some(token) => token,
            None => return Failure((Status::MethodNotAllowed, ApiKeyError::MissingToken)),
        };

        if super_admin_token == parameter_token {
            return Success(SuperAdminGuard { index: index_name });
        } else {
            return Failure((Status::MethodNotAllowed, ApiKeyError::Invalid));
        }
    }
}
