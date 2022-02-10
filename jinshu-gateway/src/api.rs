use sea_orm::JsonValue;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// HTTP 路由
pub mod route {
    /// 用户
    pub const USER: &str = "/user";
    /// 注册
    pub const SIGN_UP: &str = "/sign_up";
    /// 登录
    pub const SIGN_IN: &str = "/sign_in";
    /// 登出
    pub const SIGN_OUT: &str = "/sign_out";
}

/// 注册/创建用户请求参数
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserParam {
    /// 外系统用户ID
    pub external_id: String,
    /// 扩展字段
    pub extension: Option<JsonValue>,
}

/// 注册/创建用户返回结果
#[derive(Debug, Deserialize, Serialize)]
pub struct CreateUserResult {
    /// 新用户的锦书用户ID
    pub id: Uuid,
}

/// 登录请求参数
#[derive(Debug, Deserialize, Serialize)]
pub struct SignInParam {
    /// 锦书用户ID
    pub user_id: Uuid,
    /// 扩展字段
    #[serde(default)]
    pub extension: JsonValue,
}

/// 登录返回结果
#[derive(Debug, Deserialize, Serialize)]
pub struct SignInResult {
    /// 锦书用户ID
    pub user_id: Uuid,
    /// 用于登录的令牌
    pub token: Uuid,
    /// 扩展字段
    pub extension: JsonValue,
    /// 过期时间
    pub expire: u64,
}

/// 登出返回结果
#[derive(Debug, Deserialize, Serialize)]
pub struct SignOutParam {
    /// 锦书用户ID
    pub user_id: Uuid,
}
