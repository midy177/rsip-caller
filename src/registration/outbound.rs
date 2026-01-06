/// Outbound 代理注册实现
///
/// 完全匹配话机行为：
/// - Request-URI: sip:xfc:5060 (租户地址)
/// - From/To: sip:1001@xfc:5060 (租户域名)
/// - realm: "xfc" (租户ID)
/// - 物理发送到代理服务器
use super::traits::{Registrar, RegistrationConfig, RegistrationResult};
use async_trait::async_trait;
use rand::Rng;
use rsip::{
    headers::{CallId, ToTypedHeader},
    message::HeadersExt,
    Response, SipMessage, StatusCode, Uri,
};
use rsipstack::{
    transaction::{
        endpoint::EndpointInnerRef,
        key::{TransactionKey, TransactionRole},
        make_tag,
        transaction::Transaction,
    },
};
use tracing::{debug, error, info, warn};

/// Outbound 代理注册器
///
/// 手动构造 SIP 消息，完全匹配话机行为
pub struct OutboundProxyRegistrar {
    /// Endpoint 引用
    endpoint: EndpointInnerRef,

    /// 配置
    config: RegistrationConfig,

    /// 租户域名 (如 "xfc")
    tenant_domain: String,

    /// 代理服务器地址 (如 "sip.tst.novo-one.com:5060")
    proxy_addr: String,

    /// 是否已注册
    is_registered: bool,

    /// CSeq 计数器
    cseq: u32,

    /// From tag (保持会话一致性)
    from_tag: String,

    /// 公共地址
    public_address: Option<String>,

    /// Contact URI
    contact_uri: Option<String>,
}

impl OutboundProxyRegistrar {
    /// 创建新的 Outbound 代理注册器
    ///
    /// # 参数
    /// - `endpoint`: SIP Endpoint Inner
    /// - `config`: 注册配置
    /// - `tenant_domain`: 租户域名（用作 realm）
    /// - `proxy_addr`: 代理服务器地址（物理连接地址）
    pub fn new(
        endpoint: EndpointInnerRef,
        config: RegistrationConfig,
        tenant_domain: String,
        proxy_addr: String,
    ) -> Self {
        info!(
            "创建 Outbound 代理注册器: 用户={}, 租户={}, 代理={}",
            config.username, tenant_domain, proxy_addr
        );
        info!("完全匹配话机行为:");
        info!("  - Request-URI: sip:{}:5060 (逻辑地址)", tenant_domain);
        info!("  - From/To: sip:{}@{}", config.username, tenant_domain);
        info!("  - realm: \"{}\"", tenant_domain);
        info!("  - 物理发送到: {}", proxy_addr);

        // 生成随机 From tag
        let from_tag = make_tag().to_string();

        Self {
            endpoint,
            contact_uri: config.contact_uri.clone(),
            config,
            tenant_domain,
            proxy_addr,
            is_registered: false,
            cseq: 1,
            from_tag,
            public_address: None,
        }
    }

    /// 解析 WWW-Authenticate header 并计算 Authorization
    fn compute_authorization(
        &self,
        response: &Response,
        uri: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let www_auth = response
            .www_authenticate_header()
            .ok_or("缺少 WWW-Authenticate header")?;

        let auth_str = www_auth.to_string();
        debug!("WWW-Authenticate: {}", auth_str);

        // 提取认证参数
        let server_realm = extract_param(&auth_str, "realm").unwrap_or_default();
        let nonce = extract_param(&auth_str, "nonce")?;
        let algorithm = extract_param(&auth_str, "algorithm").unwrap_or("MD5".to_string());
        let qop = extract_param(&auth_str, "qop").ok();

        // 使用租户ID作为realm，而不是服务器返回的realm
        let realm = &self.tenant_domain;

        info!("认证信息:");
        info!("  - 服务器返回 realm: {}", server_realm);
        info!("  - 实际使用 realm: {} (租户ID)", realm);
        info!("  - nonce: {}", nonce);
        info!("  - algorithm: {}", algorithm);
        info!("  - qop: {:?}", qop);

        let username = &self.config.username;
        let password = &self.config.password;

        // 计算 HA1 和 HA2
        let ha1 = md5_hash(&format!("{}:{}:{}", username, realm, password));
        let ha2 = md5_hash(&format!("REGISTER:{}", uri));

        let response_hash = if let Some(qop_val) = qop {
            let nc = "00000001";
            let mut rng = rand::rng();
            let cnonce = format!("{:08x}", rng.random::<u32>());

            let response = md5_hash(&format!(
                "{}:{}:{}:{}:{}:{}",
                ha1, nonce, nc, cnonce, qop_val, ha2
            ));

            format!(
                "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", response=\"{}\", algorithm={}, qop=\"{}\", nc={}, cnonce=\"{}\"",
                username, realm, nonce, uri, response, algorithm, qop_val, nc, cnonce
            )
        } else {
            let response = md5_hash(&format!("{}:{}:{}", ha1, nonce, ha2));

            format!(
                "Digest username=\"{}\", realm=\"{}\", nonce=\"{}\", uri=\"{}\", response=\"{}\", algorithm={}",
                username, realm, nonce, uri, response, algorithm
            )
        };

        debug!("Authorization: {}", response_hash);
        Ok(response_hash)
    }
}

#[async_trait]
impl Registrar for OutboundProxyRegistrar {
    async fn register(&mut self, server_uri: Uri, expires: Option<u32>) -> RegistrationResult {
        self.cseq += 1;

        info!(
            "执行 Outbound 代理注册: 租户={}, From/To域名={}, 代理={}",
            server_uri, self.tenant_domain, self.proxy_addr
        );

        // 使用代理地址作为 Request-URI（物理目标）
        let proxy_uri: rsip::Uri = format!("sip:{}", self.proxy_addr).as_str().try_into()?;

        // 构造 To URI (sip:1001@xfc) - 使用租户域名
        let to_uri: rsip::Uri = format!("sip:{}@{}", self.config.username, self.tenant_domain).as_str().try_into()?;

        let to = rsip::typed::To {
            display_name: None,
            uri: to_uri.clone(),
            params: vec![],
        };

        // 构造 From URI (sip:1001@xfc;tag=xxx) - 使用租户域名
        let from_tag: rsip::param::Tag = self.from_tag.clone().try_into()?;
        let from = rsip::typed::From {
            display_name: None,
            uri: to_uri.clone(),
            params: vec![],
        }
        .with_tag(from_tag);

        // 获取 Via
        let via = self.endpoint.get_via(None, None)?;

        // 构造 Contact
        let contact_uri = self.contact_uri.as_ref().ok_or("Contact URI 未设置")?;
        let contact_uri_parsed: rsip::Uri = contact_uri.as_str().try_into()?;
        let contact = rsip::typed::Contact::from(contact_uri_parsed.clone());

        // 构造 REGISTER 请求 - Request-URI 使用代理地址
        let mut request = self.endpoint.make_request(
            rsip::Method::Register,
            proxy_uri.clone(),  // Request-URI 使用代理地址
            via,
            from,
            to,
            self.cseq,
            None,
        );

        // 添加必要的 headers
        request.headers.unique_push(self.config.call_id.as_ref().ok_or("Call-ID 未设置")?.clone().into());
        request.headers.unique_push(contact.into());
        request.headers.unique_push(
            rsip::headers::Allow::default().into()
        );
        if let Some(exp) = expires {
            request.headers.unique_push(rsip::headers::Expires::from(exp).into());
        }

        debug!("构造的 REGISTER 请求:\n{}", request);

        // 创建 Transaction 并发送
        let key = TransactionKey::from_request(&request, TransactionRole::Client)?;
        let mut tx = Transaction::new_client(key, request, self.endpoint.clone(), None);
        tx.send().await?;

        let mut auth_sent = false;

        // 接收响应循环（参考rsipstack实现）
        while let Some(msg) = tx.receive().await {
            match msg {
                SipMessage::Response(resp) => match resp.status_code {
                    StatusCode::Trying => {
                        debug!("收到 100 Trying");
                        continue;
                    }
                    StatusCode::Unauthorized | StatusCode::ProxyAuthenticationRequired => {
                        if auth_sent {
                            debug!("认证后仍收到 {} 响应", resp.status_code);
                            return Ok(resp);
                        }

                        info!("收到 {} 认证挑战，准备认证...", resp.status_code);

                        // 计算 Authorization（使用租户ID作为realm，使用代理地址作为URI）
                        let authorization = self.compute_authorization(&resp, &proxy_uri.to_string())?;

                        self.cseq += 1;

                        // 重新构造带认证的请求
                        let from_tag: rsip::param::Tag = self.from_tag.clone().try_into()?;
                        let from_with_tag = rsip::typed::From {
                            display_name: None,
                            uri: to_uri.clone(),
                            params: vec![],
                        }
                        .with_tag(from_tag);

                        let to_for_auth = rsip::typed::To {
                            display_name: None,
                            uri: to_uri.clone(),
                            params: vec![],
                        };

                        let via = self.endpoint.get_via(None, None)?;
                        let contact = rsip::typed::Contact::from(contact_uri_parsed.clone());

                        let mut auth_request = self.endpoint.make_request(
                            rsip::Method::Register,
                            proxy_uri.clone(),
                            via,
                            from_with_tag,
                            to_for_auth,
                            self.cseq,
                            None,
                        );

                        auth_request.headers.unique_push(self.config.call_id.as_ref().ok_or("Call-ID 未设置")?.clone().into());
                        auth_request.headers.unique_push(contact.into());
                        auth_request.headers.unique_push(
                            rsip::headers::Allow::from("ACK, BYE, CANCEL, INFO, INVITE, MESSAGE, NOTIFY, OPTIONS, PRACK, PUBLISH, REFER, REGISTER, SUBSCRIBE, UPDATE").into()
                        );
                        if let Some(exp) = expires {
                            auth_request.headers.unique_push(rsip::headers::Expires::from(exp).into());
                        }

                        // 添加 Authorization header
                        auth_request.headers.push(
                            rsip::Header::Authorization(authorization.try_into()?)
                        );

                        debug!("发送带认证的 REGISTER:\n{}", auth_request);

                        let key2 = TransactionKey::from_request(&auth_request, TransactionRole::Client)?;
                        tx = Transaction::new_client(key2, auth_request, self.endpoint.clone(), None);
                        tx.send().await?;
                        auth_sent = true;
                        continue;
                    }
                    StatusCode::OK => {
                        self.is_registered = true;
                        info!("✓ Outbound 代理注册成功: 200 OK");

                        // 提取公共地址
                        if let Ok(Some(contact)) = resp.contact_header().map(|c| c.typed().ok()) {
                            self.public_address = Some(contact.uri.to_string());
                            debug!("公共地址: {:?}", self.public_address);
                        }

                        return Ok(resp);
                    }
                    _ => {
                        error!("注册失败: {}", resp.status_code);
                        return Ok(resp);
                    }
                },
                _ => continue,
            }
        }

        Err("未收到响应".into())
    }

    async fn unregister(&mut self) -> RegistrationResult {
        info!("执行 Outbound 代理注销");
        warn!("Outbound 代理注销功能尚未完全实现");
        self.is_registered = false;
        Err("Outbound proxy unregister not fully implemented".into())
    }

    async fn refresh(&mut self) -> RegistrationResult {
        info!("刷新 Outbound 代理注册");
        Err("Outbound proxy refresh not fully implemented".into())
    }

    fn is_registered(&self) -> bool {
        self.is_registered
    }

    fn public_address(&self) -> Option<String> {
        self.public_address.clone()
    }

    fn contact_uri(&self) -> Option<String> {
        self.contact_uri.clone()
    }

    fn set_call_id(&mut self, call_id: CallId) {
        debug!("设置 Call-ID: {}", call_id);
        self.config.call_id = Some(call_id);
    }
}

// 辅助函数

/// 从认证字符串中提取参数
fn extract_param(auth_str: &str, param_name: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // 尝试带引号的形式: param="value"
    if let Some(start) = auth_str.find(&format!("{}=\"", param_name)) {
        let value_start = start + param_name.len() + 2;
        if let Some(end) = auth_str[value_start..].find('"') {
            return Ok(auth_str[value_start..value_start + end].to_string());
        }
    }

    // 尝试不带引号的形式: param=value
    if let Some(start) = auth_str.find(&format!("{}=", param_name)) {
        let value_start = start + param_name.len() + 1;
        let value_end = auth_str[value_start..]
            .find(&[',', ' ', '\r', '\n'][..])
            .unwrap_or(auth_str[value_start..].len());
        return Ok(auth_str[value_start..value_start + value_end].to_string());
    }

    Err(format!("未找到参数: {}", param_name).into())
}

/// 计算 MD5 哈希
fn md5_hash(input: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}
