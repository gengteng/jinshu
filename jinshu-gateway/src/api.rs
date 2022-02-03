use sea_orm::JsonValue;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod route {
    pub const USER: &str = "/user";
    pub const SIGN_UP: &str = "/sign_up";
    pub const SIGN_IN: &str = "/sign_in";
    pub const SIGN_OUT: &str = "/sign_out";
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserParam {
    pub external_id: String,
    pub extension: Option<JsonValue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserResult {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignInParam {
    pub user_id: Uuid,
    #[serde(default)]
    pub extension: JsonValue,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignInResult {
    pub user_id: Uuid,
    pub token: Uuid,
    pub extension: JsonValue,
    pub expire: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SignOutParam {
    pub user_id: Uuid,
}
