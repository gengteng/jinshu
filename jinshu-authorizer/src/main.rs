use deadpool_redis::redis::AsyncCommands;
use jinshu_authorizer::config::AuthorizerConfig;
use jinshu_common::Config;
use jinshu_redis::config::RedisConfig;
use jinshu_redis::get_sign_in_key;
use jinshu_rpc::authorizer::authorizer_server::AuthorizerServer;
use jinshu_rpc::authorizer::{authorizer_server, SignInResult, Token};
use jinshu_rpc::registry::etcd::{EtcdConfig, EtcdRegistry};
use jinshu_rpc::registry::Registry;
use jinshu_rpc::{internal, invalid_argument};
use jinshu_tracing::config::TracingConfig;
use jinshu_utils::shutdown_signal;
use serde::{Deserialize, Serialize};
use tonic::{Request, Response, Status};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
struct Conf {
    tracing: TracingConfig,
    etcd: EtcdConfig,
    redis: RedisConfig,
    authorizer: AuthorizerConfig,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conf = Conf::from_cli()?;

    let _tracer = conf.tracing.init("authorizer")?;

    let Conf {
        etcd,
        redis,
        authorizer: AuthorizerConfig { service },
        ..
    } = conf;

    tracing::info!(?etcd);
    let registry = EtcdRegistry::new(&etcd).await?;

    tracing::info!(?redis);
    let redis_config: deadpool_redis::Config = redis.into();
    let redis = redis_config.builder()?.build()?;

    let authorizer = AuthorizerServer::new(Authorizer::new(redis));

    let (uri, handle) = registry
        .run_service(service, authorizer, shutdown_signal())
        .await?;

    tracing::info!(%uri, "Authorizer service is running.");
    handle.await?;

    Ok(())
}

pub struct Authorizer {
    redis: deadpool_redis::Pool,
}

impl Authorizer {
    pub fn new(redis: deadpool_redis::Pool) -> Self {
        Self { redis }
    }
}

#[tonic::async_trait]
impl authorizer_server::Authorizer for Authorizer {
    #[tracing::instrument(skip(self, request))]
    async fn sign_in(&self, request: Request<Token>) -> Result<Response<SignInResult>, Status> {
        let Token { user_id, token } = request.into_inner();
        tracing::info!(user_id = %user_id, token = %token);

        let user_id: Uuid = user_id.parse().map_err(invalid_argument)?;
        let token: Uuid = token.parse().map_err(invalid_argument)?;

        let mut conn = self.redis.get().await.map_err(internal)?;

        let key = get_sign_in_key(user_id.as_simple());

        let value: Option<String> = conn.get(&key).await.map_err(internal)?;

        tracing::info!(%key, ?value);

        #[derive(Debug, Deserialize)]
        struct SignIn {
            user_id: Uuid,
            token: Uuid,
            extension: serde_json::Value,
        }

        let sign_in: SignIn = match value {
            None => {
                return Ok(Response::new(SignInResult {
                    ok: false,
                    extension: None,
                }));
            }
            Some(value) => serde_json::from_str(&value).map_err(internal)?,
        };

        tracing::info!(?sign_in);

        let result = SignInResult {
            ok: sign_in.user_id == user_id && sign_in.token == token,
            extension: Some(sign_in.extension.to_string()),
        };

        Ok(Response::new(result))
    }
}
