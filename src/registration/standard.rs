/// 标准注册实现
///
/// 使用 rsipstack 内置的 Registration 类
use super::traits::{Registrar, RegistrationConfig, RegistrationResult};
use async_trait::async_trait;
use rsip::{headers::CallId, Uri};
use rsipstack::{
    dialog::{authenticate::Credential, registration::Registration},
    transaction::endpoint::EndpointInnerRef,
};
use tracing::{debug, info, warn};

/// 标准注册器
///
/// 适用于直接连接到 SIP 服务器的场景
pub struct StandardRegistrar {
    /// rsipstack 的 Registration 实例
    registration: Registration,

    /// 配置
    config: RegistrationConfig,

    /// 是否已注册
    is_registered: bool,
}

impl StandardRegistrar {
    /// 创建新的标准注册器
    pub fn new(endpoint: EndpointInnerRef, config: RegistrationConfig) -> Self {
        info!("创建标准注册器: 用户 {}", config.username);

        // 创建认证凭证
        let credential = Credential {
            username: config.username.clone(),
            password: config.password.clone(),
            realm: config.realm.clone(),
        };

        // 创建 Registration 实例
        let mut registration = Registration::new(endpoint, Some(credential));

        // 设置 Call-ID（如果提供）
        if let Some(call_id) = &config.call_id {
            registration.call_id = call_id.clone();
        }

        Self {
            registration,
            config,
            is_registered: false,
        }
    }
}

#[async_trait]
impl Registrar for StandardRegistrar {
    async fn register(&mut self, server_uri: Uri, expires: Option<u32>) -> RegistrationResult {
        let expires = expires.unwrap_or(self.config.default_expires);

        info!("执行标准注册: server={}, expires={}", server_uri, expires);
        debug!("用户: {}, realm: {:?}", self.config.username, self.config.realm);

        match self.registration.register(server_uri, Some(expires)).await {
            Ok(response) => {
                if response.status_code == rsip::StatusCode::OK {
                    self.is_registered = true;
                    info!("✓ 标准注册成功: {}", response.status_code);
                } else {
                    info!("标准注册响应: {}", response.status_code);
                }
                Ok(response)
            }
            Err(e) => {
                self.is_registered = false;
                Err(Box::new(e))
            }
        }
    }

    async fn unregister(&mut self) -> RegistrationResult {
        info!("执行标准注销");

        // 注销功能需要完整实现
        // 需要保存原始 server URI
        warn!("标准注销功能尚未完全实现");
        self.is_registered = false;

        Err("Standard unregister not fully implemented".into())
    }

    async fn refresh(&mut self) -> RegistrationResult {
        info!("刷新标准注册");

        // 刷新注册需要知道原始的 server URI
        // 这里简化处理，假设已经存储了 server URI
        // 实际应用中需要在首次注册时保存 server URI

        Err("Standard registrar refresh not implemented yet".into())
    }

    fn is_registered(&self) -> bool {
        self.is_registered
    }

    fn public_address(&self) -> Option<String> {
        self.registration
            .public_address
            .as_ref()
            .map(|addr| addr.to_string())
    }

    fn contact_uri(&self) -> Option<String> {
        self.registration
            .contact
            .as_ref()
            .map(|contact| contact.uri.to_string())
    }

    fn set_call_id(&mut self, call_id: CallId) {
        self.registration.call_id = call_id;
    }
}
