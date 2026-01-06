/// Registration traits 定义
///
/// 使用 trait 抽象不同的注册策略
use rsip::{Response, Uri, headers::CallId};
use async_trait::async_trait;

/// 注册结果
pub type RegistrationResult = Result<Response, Box<dyn std::error::Error + Send + Sync>>;

/// 注册器 trait
///
/// 定义 SIP 注册的核心行为
#[async_trait]
pub trait Registrar: Send + Sync {
    /// 执行注册
    ///
    /// # 参数
    /// - `server_uri`: 注册服务器 URI
    /// - `expires`: 过期时间（秒）
    ///
    /// # 返回
    /// - `Ok(Response)`: 注册响应
    /// - `Err`: 注册失败
    async fn register(&mut self, server_uri: Uri, expires: Option<u32>) -> RegistrationResult;

    /// 执行注销
    ///
    /// 发送 expires=0 的 REGISTER 请求
    async fn unregister(&mut self) -> RegistrationResult;

    /// 刷新注册
    ///
    /// 重新发送注册请求以延长过期时间
    async fn refresh(&mut self) -> RegistrationResult;

    /// 获取注册状态
    fn is_registered(&self) -> bool;

    /// 获取公共地址
    ///
    /// NAT 穿越后服务器看到的地址
    fn public_address(&self) -> Option<String>;

    /// 获取 Contact URI
    fn contact_uri(&self) -> Option<String>;

    /// 设置 Call-ID
    fn set_call_id(&mut self, call_id: CallId);
}

/// 注册配置
#[derive(Debug, Clone)]
pub struct RegistrationConfig {
    /// 用户名
    pub username: String,

    /// 密码
    pub password: String,

    /// 认证域 (realm)
    pub realm: Option<String>,

    /// Contact URI
    pub contact_uri: Option<String>,

    /// Call-ID
    pub call_id: Option<CallId>,

    /// 默认过期时间
    pub default_expires: u32,
}

impl RegistrationConfig {
    /// 创建新的注册配置
    pub fn new(username: String, password: String) -> Self {
        Self {
            username,
            password,
            realm: None,
            contact_uri: None,
            call_id: None,
            default_expires: 3600,
        }
    }

    /// 设置 realm
    pub fn with_realm(mut self, realm: String) -> Self {
        self.realm = Some(realm);
        self
    }

    /// 设置 Contact URI
    pub fn with_contact_uri(mut self, contact_uri: String) -> Self {
        self.contact_uri = Some(contact_uri);
        self
    }

    /// 设置 Call-ID
    pub fn with_call_id(mut self, call_id: CallId) -> Self {
        self.call_id = Some(call_id);
        self
    }

    /// 设置过期时间
    pub fn with_expires(mut self, expires: u32) -> Self {
        self.default_expires = expires;
        self
    }
}
