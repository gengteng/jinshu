//! 用户长链接 Session 存储

use crate::config::RedisConfig;
use deadpool_redis::redis::AsyncCommands;
use std::fmt::Display;
use uuid::Uuid;

#[derive(Clone)]
pub struct SessionStore {
    redis: deadpool_redis::Pool,
}

fn get_user_session_key<D: Display>(user_id: D) -> String {
    format!("user:session:{}", user_id)
}

impl SessionStore {
    pub async fn new(config: RedisConfig) -> crate::Result<Self> {
        let redis_config: deadpool_redis::Config = config.into();
        let redis = redis_config.builder()?.build()?;
        let session_store = SessionStore::from_pool(redis);

        Ok(session_store)
    }

    pub fn from_pool(redis: deadpool_redis::Pool) -> Self {
        Self { redis }
    }

    pub async fn store(&self, user_id: Uuid, service_key: &str) -> crate::Result<()> {
        let mut conn = self.redis.get().await?;
        let _: () = conn.set(get_user_session_key(user_id), service_key).await?;
        Ok(())
    }

    pub async fn load(&self, user_id: Uuid) -> crate::Result<Option<String>> {
        let mut conn = self.redis.get().await?;
        let endpoint: Option<String> = conn.get(get_user_session_key(user_id)).await?;
        Ok(endpoint)
    }

    pub async fn remove(&self, user_id: Uuid) -> crate::Result<()> {
        let mut conn = self.redis.get().await?;
        let _: () = conn.del(get_user_session_key(user_id)).await?;
        Ok(())
    }
}
