use axum::extract::{Extension, Json, Path};
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{AddExtensionLayer, Router};
use deadpool_redis::{redis::AsyncCommands, Pool as RedisPool};
use jinshu_common::Config;
use jinshu_database::config::DatabaseConfig;
use jinshu_database::prelude::*;
use jinshu_database::user::Model as UserModel;
use jinshu_gateway::{
    config::GatewayConfig, route, CreateUserParam, CreateUserResult, SignInParam, SignInResult,
    SignOutParam,
};
use jinshu_redis::{config::RedisConfig, get_sign_in_key};
use jinshu_tracing::config::TracingConfig;
use jinshu_utils::{current_millisecond, shutdown_signal};
use sea_orm::{ActiveModelTrait, EntityTrait};
use sea_orm::{Database, DatabaseConnection, Set};
use serde::Deserialize;
use std::fmt::Display;
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct Conf {
    tracing: TracingConfig,
    gateway: GatewayConfig,
    database: DatabaseConfig,
    redis: RedisConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Conf::from_cli()?;

    let _tracer = conf.tracing.init("gateway")?;

    let Conf {
        gateway: GatewayConfig { ip, port },
        database,
        redis,
        ..
    } = conf;

    tracing::info!(?database);
    let database = Database::connect(database).await?;

    tracing::info!(?redis);
    let redis_config: deadpool_redis::Config = redis.into();
    let redis = redis_config.builder()?.build()?;

    let app = Router::new()
        .route(route::USER, post(create_user))
        .route(route::SIGN_UP, post(create_user)) // alias for create user
        .route(route::USER, get(retrieve_user))
        .route(route::SIGN_IN, post(sign_in))
        .route(route::SIGN_OUT, delete(sign_out))
        .layer(AddExtensionLayer::new(database))
        .layer(AddExtensionLayer::new(redis))
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let addr = SocketAddr::new(ip, port);

    tracing::info!(%addr, "jinshu-gateway is started.");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

#[tracing::instrument(skip_all)]
async fn create_user(
    Extension(db): Extension<DatabaseConnection>,
    Json(create): Json<CreateUserParam>,
) -> Result<(StatusCode, axum::Json<CreateUserResult>), (StatusCode, String)> {
    tracing::info!(?create);
    let id = uuid::Uuid::new_v4();
    let model = jinshu_database::user::ActiveModel {
        id: Set(id.as_simple().to_string()),
        external_id: Set(create.external_id),
        extension: Set(create.extension),
        ..Default::default()
    };

    model.insert(&db).await.map_err(internal_error)?;

    Ok((StatusCode::CREATED, Json(CreateUserResult { id })))
}

#[tracing::instrument(skip_all)]
async fn retrieve_user(
    Extension(db): Extension<DatabaseConnection>,
    Path(user_id): Path<Uuid>,
) -> Result<axum::Json<UserModel>, (StatusCode, String)> {
    tracing::info!(?user_id);
    match User::find_by_id(user_id.as_simple().to_string())
        .one(&db)
        .await
        .map_err(internal_error)?
    {
        Some(user) => Ok(Json(user)),
        None => Err((StatusCode::NOT_FOUND, "".into())),
    }
}

#[tracing::instrument(skip_all)]
async fn sign_in(
    Extension(db): Extension<DatabaseConnection>,
    Extension(redis): Extension<RedisPool>,
    Json(param): Json<SignInParam>,
) -> Result<axum::Json<SignInResult>, (StatusCode, String)> {
    tracing::info!(?param);
    const TOKEN_VALIDITY_SEC: usize = 300;

    let user = match User::find_by_id(param.user_id.as_simple().to_string())
        .one(&db)
        .await
        .map_err(internal_error)?
    {
        Some(user) => user,
        None => return Err((StatusCode::NOT_FOUND, "".into())),
    };

    let token = Uuid::new_v4();
    let sign_in = SignInResult {
        user_id: user.id.parse().map_err(internal_error)?,
        token,
        extension: param.extension,
        expire: current_millisecond() + (TOKEN_VALIDITY_SEC as u64 * 1000),
    };

    let mut conn = redis.get().await.map_err(internal_error)?;

    let a: String = conn
        .set_ex(
            get_sign_in_key(sign_in.user_id.as_simple()),
            serde_json::to_string(&sign_in).map_err(internal_error)?,
            TOKEN_VALIDITY_SEC,
        )
        .await
        .map_err(internal_error)?;

    tracing::debug!("set result: {}", a);

    Ok(Json(sign_in))
}

#[tracing::instrument(skip_all)]
async fn sign_out(
    Json(param): Json<SignOutParam>,
    Extension(redis): Extension<deadpool_redis::Pool>,
) -> Result<(StatusCode, Json<()>), (StatusCode, String)> {
    tracing::info!(?param);
    let mut conn = redis.get().await.map_err(internal_error)?;

    let _: i64 = conn
        .del(get_sign_in_key(param.user_id.as_simple()))
        .await
        .map_err(internal_error)?;

    Ok((StatusCode::OK, Json(())))
}

fn internal_error<E: Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
