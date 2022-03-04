mod model;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{PasswordHash, PasswordHasher, PasswordVerifier};
use axum::extract;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::routing::{delete, post};
use axum::{Json, Router};
use deadpool_redis::redis::AsyncCommands;
use jinshu_common::Config;
use jinshu_database::config::DatabaseConfig;
use jinshu_gateway::{
    route, CreateUserParam, CreateUserResult, SignInParam, SignInResult, SignOutParam,
};
use jinshu_redis::config::RedisConfig;
use jinshu_tracing::config::TracingConfig;
use jinshu_utils::secret::Secret;
use jinshu_utils::shutdown_signal;
use model::app_user;
use reqwest::{Client, Url};
use sea_orm::{ActiveModelTrait, JsonValue};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use sea_orm::{Database, DatabaseConnection, Set};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, SocketAddr};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct AppServerConfig {
    ip: IpAddr,
    port: u16,
    gateway_host: String,
    gateway_port: u16,
}

impl Default for AppServerConfig {
    fn default() -> Self {
        Self {
            ip: [0u8, 0, 0, 0].into(),
            port: 8765,
            gateway_host: "127.0.0.1".into(),
            gateway_port: 9200,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Conf {
    tracing: TracingConfig,
    database: DatabaseConfig,
    redis: RedisConfig,
    app: AppServerConfig,
}

#[derive(Clone)]
struct Resources {
    base_url: Url,
    http: Client,
    database: DatabaseConnection,
    redis: deadpool_redis::Pool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Conf::from_cli()?;

    let _tracer = conf.tracing.init("app-server");

    let Conf {
        database,
        redis,
        app:
            AppServerConfig {
                ip,
                port,
                gateway_host,
                gateway_port,
            },
        ..
    } = conf;

    let base_url: Url = format!("http://{}:{}", gateway_host, gateway_port).parse()?;

    let http = reqwest::Client::new();

    tracing::info!(?database);
    let database = Database::connect(database).await?;

    tracing::info!(?redis);
    let redis_config: deadpool_redis::Config = redis.into();
    let redis = redis_config.builder()?.build()?;

    let resources = Resources {
        base_url,
        http,
        database,
        redis,
    };

    let router = Router::new()
        .route("/sign_up", post(sign_up))
        .route("/sign_in", post(sign_in))
        .route("/sign_out", delete(sign_out))
        .layer(Extension(resources))
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let addr = SocketAddr::from((ip, port));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

#[derive(Debug, serde_repr::Serialize_repr, serde_repr::Deserialize_repr)]
#[repr(u8)]
enum Gender {
    Unknown = 0,
    Male = 1,
    Female = 2,
}

impl Display for Gender {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Default for Gender {
    fn default() -> Self {
        Gender::Unknown
    }
}

#[derive(Debug, Deserialize)]
struct AppSignUpParam {
    username: String,
    password: Secret,
    #[serde(default)]
    gender: Gender,
}

#[derive(Debug, Serialize)]
struct AppSignUpResult {
    id: Uuid,
}

#[tracing::instrument(skip_all)]
async fn sign_up(
    extract::Json(param): extract::Json<AppSignUpParam>,
    Extension(resources): Extension<Resources>,
) -> Result<(StatusCode, Json<AppSignUpResult>), (StatusCode, String)> {
    tracing::info!(?param.username);
    let user_id = Uuid::new_v4();

    let password = tokio::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        argon2::Argon2::default()
            .hash_password(param.password.expose().as_bytes(), &salt)
            .map(|h| h.to_string())
    })
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;

    let model = app_user::ActiveModel {
        id: Set(user_id.as_simple().to_string()),
        username: Set(param.username),
        password: Set(password),
        gender: Set(param.gender as i32),
        ..Default::default()
    };

    model
        .insert(&resources.database)
        .await
        .map_err(internal_error)?;

    let url = resources
        .base_url
        .join(route::USER)
        .map_err(internal_error)?;
    let resp = resources
        .http
        .post(url)
        .json(&CreateUserParam {
            external_id: user_id.as_simple().to_string(),
            extension: None,
        })
        .send()
        .await
        .map_err(internal_error)?;

    let status = resp.status();
    if status.is_success() {
        let result: CreateUserResult = resp.json().await.map_err(internal_error)?;
        let model = app_user::ActiveModel {
            id: Set(user_id.as_simple().to_string()),
            jinshu_id: Set(Some(result.id.as_simple().to_string())),
            ..Default::default()
        };
        model
            .update(&resources.database)
            .await
            .map_err(internal_error)?;
        Ok((StatusCode::CREATED, Json(AppSignUpResult { id: user_id })))
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Sign up error: {}",
                resp.text().await.map_err(internal_error)?
            ),
        ))
    }
}

#[derive(Debug, Deserialize)]
struct AppSignInParam {
    username: String,
    password: Secret,
}

#[derive(Debug, Serialize)]
struct AppSignInResult {
    id: Uuid,
    token: Uuid,
    jinshu: SignInResult,
}

#[tracing::instrument(skip_all)]
async fn sign_in(
    extract::Json(param): extract::Json<AppSignInParam>,
    Extension(resources): Extension<Resources>,
) -> Result<Json<AppSignInResult>, (StatusCode, String)> {
    tracing::info!(?param);
    let query: Option<app_user::Model> = app_user::Entity::find()
        .filter(app_user::Column::Username.eq(param.username.as_str()))
        .one(&resources.database)
        .await
        .map_err(internal_error)?;

    if let Some(app_user) = query {
        let app_user_id: Uuid = app_user.id.parse().map_err(internal_error)?;

        let valid = tokio::task::spawn_blocking(move || {
            let hash = match PasswordHash::new(&app_user.password) {
                Ok(hash) => hash,
                Err(error) => {
                    tracing::error!(%error, "Failed to create PasswordHash");
                    return false;
                }
            };
            argon2::Argon2::default()
                .verify_password(param.password.expose().as_bytes(), &hash)
                .is_ok()
        })
        .await
        .map_err(internal_error)?;

        if valid {
            if let Some(jinshu_id) = app_user.jinshu_id {
                let url = resources
                    .base_url
                    .join(route::SIGN_IN)
                    .map_err(internal_error)?;
                let resp = resources
                    .http
                    .post(url)
                    .json(&SignInParam {
                        user_id: jinshu_id.parse().map_err(internal_error)?,
                        extension: JsonValue::Null,
                    })
                    .send()
                    .await
                    .map_err(internal_error)?;

                let status = resp.status();
                if status.is_success() {
                    let result: SignInResult = resp.json().await.map_err(internal_error)?;
                    let token = Uuid::new_v4();

                    let mut conn = resources.redis.get().await.map_err(internal_error)?;
                    let _: String = conn
                        .set(
                            get_app_sign_in_key(&app_user_id),
                            token.as_simple().to_string(),
                        )
                        .await
                        .map_err(internal_error)?;

                    Ok(Json(AppSignInResult {
                        id: app_user_id,
                        token,
                        jinshu: result,
                    }))
                } else {
                    let body = resp.text().await.map_err(internal_error)?;
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to sign in IM server: {}", body),
                    ))
                }
            } else {
                Err((
                    StatusCode::NOT_IMPLEMENTED,
                    "User doesn't have a jinshu ID.".into(),
                ))
            }
        } else {
            Err((StatusCode::BAD_REQUEST, "Invalid password".into()))
        }
    } else {
        Err((StatusCode::NOT_FOUND, "Invalid username".into()))
    }
}

#[derive(Debug, Deserialize)]
struct AppSignOutParam {
    id: Uuid,
    token: Uuid,
}

#[tracing::instrument(skip_all)]
async fn sign_out(
    extract::Json(param): extract::Json<AppSignOutParam>,
    Extension(resources): Extension<Resources>,
) -> Result<(StatusCode, Json<()>), (StatusCode, String)> {
    tracing::info!(?param);
    let query: Option<app_user::Model> =
        app_user::Entity::find_by_id(param.id.as_simple().to_string())
            .one(&resources.database)
            .await
            .map_err(internal_error)?;

    if let Some(app_user) = query {
        let mut conn = resources.redis.get().await.map_err(internal_error)?;
        let key = get_app_sign_in_key(&param.id);
        let token: Option<String> = conn.get(&key).await.map_err(internal_error)?;
        if let Some(token) = token {
            let token: Uuid = token.parse().map_err(internal_error)?;
            if token != param.token {
                return Err((StatusCode::UNAUTHORIZED, "Invalid token".into()));
            }

            let _: i64 = conn.del(&key).await.map_err(internal_error)?;

            if let Some(jinshu_id) = app_user.jinshu_id {
                let url = resources
                    .base_url
                    .join(route::SIGN_OUT)
                    .map_err(internal_error)?;
                let resp = resources
                    .http
                    .delete(url)
                    .json(&SignOutParam {
                        user_id: jinshu_id.parse().map_err(internal_error)?,
                    })
                    .send()
                    .await
                    .map_err(internal_error)?;

                let status = resp.status();
                if !status.is_success() {
                    return Err((status, resp.text().await.map_err(internal_error)?));
                }
            }

            Ok((StatusCode::OK, Json(())))
        } else {
            Err((StatusCode::UNAUTHORIZED, "".into()))
        }
    } else {
        Err((StatusCode::NOT_FOUND, "".into()))
    }
}

fn internal_error<E: Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

pub fn get_app_sign_in_key<D: Display>(user_id: D) -> String {
    format!("app_user:sign_in:{}", user_id)
}
