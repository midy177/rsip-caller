/// SIP Registration Client (参考 rsipstack 实现)
///
/// 提供 SIP REGISTER 功能，支持自动代理检测
use rsip::{prelude::HeadersExt, headers::ToTypedHeader, Response, SipMessage, StatusCode};
use rsipstack::{
    dialog::authenticate::{handle_client_authenticate, Credential},
    transaction::{
        endpoint::EndpointInnerRef,
        key::{TransactionKey, TransactionRole},
        make_call_id, make_tag,
        transaction::Transaction,
    },
    Result,
};
use rsipstack::dialog::DialogId;
use tracing::{debug, info};

/// SIP Registration Client
pub struct Registration {
    pub last_seq: u32,
    pub endpoint: EndpointInnerRef,
    pub credential: Option<Credential>,
    pub contact: Option<rsip::typed::Contact>,
    pub allow: rsip::headers::Allow,
    pub public_address: Option<rsip::HostWithPort>,
    pub call_id: rsip::headers::CallId,
    /// Outbound 代理地址（可选）
    pub proxy_addr: Option<String>,
}

impl Registration {
    /// 创建新的注册客户端
    ///
    /// # 参数
    /// - `endpoint`: SIP Endpoint
    /// - `credential`: 认证凭证
    /// - `proxy_addr`: 可选的 Outbound 代理地址
    /// - `call_id`: 可选的 Call-ID header，如果为 None 则自动生成
    pub fn new(
        endpoint: EndpointInnerRef,
        credential: Option<Credential>,
        proxy_addr: Option<String>,
        call_id: Option<rsip::headers::CallId>,
    ) -> Self {
        let call_id = call_id.unwrap_or_else(|| make_call_id(endpoint.option.callid_suffix.as_deref()));
        Self {
            last_seq: 0,
            endpoint,
            credential,
            contact: None,
            allow: Default::default(),
            public_address: None,
            call_id,
            proxy_addr,
        }
    }

    /// 执行 SIP 注册（参考 rsipstack 实现，支持自动代理检测）
    ///
    /// # 参数
    /// - `server`: 服务器 URI (如 sip:xfc:5060)
    /// - `expires`: 过期时间（秒）
    ///
    /// # 返回
    /// - 成功返回 200 OK 响应
    pub async fn register(&mut self, server: rsip::Uri, expires: Option<u32>) -> Result<Response> {
        self.last_seq += 1;

        // 构造 To（完全参考 rsipstack 实现）
        let mut to = rsip::typed::To {
            display_name: None,
            uri: server.clone(),
            params: vec![],
        };

        if let Some(cred) = &self.credential {
            to.uri.auth = Some(rsip::auth::Auth {
                user: cred.username.clone(),
                password: None,
            });
        }

        // 构造 From（完全参考 rsipstack 实现）
        let from = rsip::typed::From {
            display_name: None,
            uri: to.uri.clone(),
            params: vec![],
        }
        .with_tag(make_tag());

        let via = self.endpoint.get_via(None, None)?;

        // Contact 地址选择优先级（参考 rsipstack）
        let contact = self.contact.clone().unwrap_or_else(|| {
            let contact_host_with_port = self
                .public_address
                .clone()
                .unwrap_or_else(|| via.uri.host_with_port.clone());
            rsip::typed::Contact {
                display_name: None,
                uri: rsip::Uri {
                    auth: to.uri.auth.clone(),
                    scheme: Some(rsip::Scheme::Sip),
                    host_with_port: contact_host_with_port,
                    params: vec![],
                    headers: vec![],
                },
                params: vec![],
            }
        });

        // ★ 核心改动：根据是否配置代理自动选择 Request-URI
        let request_uri = if let Some(ref proxy) = self.proxy_addr {
            info!("使用 Outbound 代理模式: proxy={}", proxy);
            format!("sip:{}", proxy).as_str().try_into()?
        } else {
            info!("使用标准注册模式");
            server.clone()
        };

        // 构造 REGISTER 请求（参考 rsipstack 实现）
        let mut request = self.endpoint.make_request(
            rsip::Method::Register,
            request_uri, // ★ 使用自动选择的 Request-URI
            via,
            from,
            to,
            self.last_seq,
            None,
        );

        // 添加必要的 headers（参考 rsipstack 实现）
        request.headers.unique_push(self.call_id.clone().into());
        request.headers.unique_push(contact.into());
        request.headers.unique_push(self.allow.clone().into());
        if let Some(expires) = expires {
            request
                .headers
                .unique_push(rsip::headers::Expires::from(expires).into());
        }

        let key = TransactionKey::from_request(&request, TransactionRole::Client)?;
        let mut tx = Transaction::new_client(key, request, self.endpoint.clone(), None);

        tx.send().await?;
        let mut auth_sent = false;

        // 接收响应循环（完全参考 rsipstack 实现）
        while let Some(msg) = tx.receive().await {
            match msg {
                SipMessage::Response(resp) => match resp.status_code {
                    StatusCode::Trying => {
                        continue;
                    }
                    StatusCode::ProxyAuthenticationRequired | StatusCode::Unauthorized => {
                        if auth_sent {
                            debug!("received {} response after auth sent", resp.status_code);
                            return Ok(resp);
                        }

                        if let Some(cred) = &self.credential {
                            self.last_seq += 1;

                            // ★ 使用 rsipstack 的 handle_client_authenticate
                            tx = handle_client_authenticate(self.last_seq, tx, resp, cred).await?;

                            tx.send().await?;
                            auth_sent = true;
                            continue;
                        } else {
                            debug!("received {} response without credential", resp.status_code);
                            return Ok(resp);
                        }
                    }
                    StatusCode::OK => {
                        // 更新 contact 和 public_address（参考 rsipstack 实现）
                        match resp.contact_header() {
                            Ok(contact) => {
                                self.contact = contact.typed().ok();
                            }
                            Err(_) => {}
                        };

                        info!(
                            "registration success: {:?} {:?}",
                            resp.status_code,
                            self.contact.as_ref().map(|c| c.uri.to_string())
                        );
                        return Ok(resp);
                    }
                    _ => {
                        info!("registration done: {:?}", resp.status_code);
                        return Ok(resp);
                    }
                },
                _ => break,
            }
        }

        Err(rsipstack::Error::DialogError(
            "registration transaction is already terminated".to_string(),
            DialogId::try_from(&tx.original)?,
            StatusCode::BadRequest,
        ))
    }
}
