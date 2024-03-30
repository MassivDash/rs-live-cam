use actix_session::Session;
use actix_web::HttpResponse;
use serde::{Deserialize, Serialize};
use std::env;
extern crate dotenv;

use dotenv::dotenv;

#[derive(Deserialize, PartialEq, Eq, Debug)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct User {
    pub id: i64,
    username: String,
    password: String,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl User {
    pub fn authenticate(credentials: Credentials) -> Result<Self, AuthError> {


        dotenv().ok();

        let password = env::var("PASSWORD").expect("PASSWORD must be set");
        let username = env::var("USERNAME").expect("USERNAME must be set");

        if *credentials.username != username {
            return Err(AuthError::InvalidCredentials(anyhow::anyhow!(
                "Invalid credentials."
            )));
        }
        
        
        if *credentials.password != password {
            return Err(AuthError::InvalidCredentials(anyhow::anyhow!(
                "Invalid credentials."
            )));
        }

        Ok(User {
            id: 42,
            username: credentials.username,
            password: credentials.password,
        })
    }
}

pub fn validate_session(session: &Session) -> Result<i64, HttpResponse> {
    println!("session: {:?}", session.entries());
    let user_id: Option<i64> = session.get("user_id").unwrap_or(None);
    println!("user_id: {:?}", user_id);

    match user_id {
        Some(id) => {
            // keep the user's session alive
            session.renew();
            Ok(id)
        }
        None => Err(HttpResponse::Unauthorized().json("Unauthorized")),
    }
}
