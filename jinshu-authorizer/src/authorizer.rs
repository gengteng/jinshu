use deadpool_redis::redis::AsyncCommands;
use jinshu_redis::get_sign_in_key;
use jinshu_rpc::authorizer::{authorizer_server, SignInResult, Token};
use jinshu_rpc::{internal, invalid_argument};
use serde::Deserialize;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// 基于 Redis 的授权服务
pub struct Authorizer {
    redis: deadpool_redis::Pool,
}

impl Authorizer {
    /// 使用 Redis Pool 创建授权服务
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
