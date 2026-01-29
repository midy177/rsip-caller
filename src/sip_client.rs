/// SIP 客户端核心模块
///
/// 提供高层次的SIP客户端功能封装
use crate::error::CallError;
use crate::sip_transport::create_transport_connection;
use rsipstack::{
    dialog::{
        authenticate::Credential, dialog_layer::DialogLayer, invitation::InviteOption,
        registration::Registration,
    },
    transaction::Endpoint,
    transport::{SipAddr, TransportLayer},
    EndpointBuilder,
};
use std::sync::Arc;
use std::time::Duration;
use rsip::Response;
use rsipstack::dialog::client_dialog::ClientInviteDialog;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use crate::error::CallResult;

/// SIP 客户端配置
pub struct SipClientConfig {
    /// 服务器 URI (例如 "sip:example.com:5060" 或 "sip:server:5060;transport=tcp")
    pub server: rsip::Uri,

    /// Outbound 代理 URI（可选）
    /// 完整URI格式，如 "sip:proxy.example.com:5060;transport=udp;lr"
    pub outbound_proxy: Option<rsip::Uri>,

    /// SIP 用户名
    pub username: String,

    /// SIP 密码
    pub password: String,

    /// User-Agent字符串
    pub user_agent: String,
}

/// SIP 客户端
pub struct SipClient {
    config: SipClientConfig,
    endpoint: Endpoint,
    dialog_layer: Arc<DialogLayer>,
    cancel_token: CancellationToken,
}

impl SipClient {
    /// 创建新的SIP客户端
    pub async fn new(config: SipClientConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let cancel_token = CancellationToken::new();

        // 获取本地IP
        let local_ip = crate::utils::get_first_non_loopback_interface()?;
        info!(
            "检测到本地出口IP: {} ({})",
            local_ip,
            if local_ip.is_ipv6() { "IPv6" } else { "IPv4" }
        );

        // 创建传输层
        let mut transport_layer = TransportLayer::new(cancel_token.clone());

        // 确定实际使用的 protocol 和连接目标
        let (protocol, connection_target) = if let Some(ref outbound_proxy) = config.outbound_proxy
        {
            // 有outbound_proxy：从proxy URI中提取transport
            let protocol = crate::utils::extract_protocol_from_uri(outbound_proxy);
            (protocol, outbound_proxy.host_with_port.to_string())
        } else {
            // 没有outbound_proxy：从server URI中提取transport
            let protocol = crate::utils::extract_protocol_from_uri(&config.server);
            (protocol, config.server.host_with_port.to_string())
        };

        // 如果有outbound代理，设置TransportLayer的outbound字段
        if let Some(ref outbound_proxy) = config.outbound_proxy {
            // 从URI中提取host:port作为连接目标
            let target = outbound_proxy.host_with_port.to_string();

            // 创建SipAddr用于outbound配置
            let sip_addr = SipAddr {
                r#type: Some(protocol.into()),
                addr: outbound_proxy.host_with_port.clone(),
            };

            // 设置TransportLayer的outbound字段
            transport_layer.outbound = Some(sip_addr);

            info!(
                "配置 Outbound 代理: {} (transport: {})",
                target,
                protocol.as_str()
            );
        }

        // 使用提取出的protocol创建传输连接
        let local_addr = format!("{}:{}", local_ip, 0).parse()?;
        let connection = create_transport_connection(
            protocol,
            local_addr,
            &connection_target,
            cancel_token.clone(),
        )
        .await?;

        transport_layer.add_transport(connection);

        // 创建端点
        let mut endpoint_builder = EndpointBuilder::new();
        endpoint_builder
            .with_cancel_token(cancel_token.clone())
            .with_transport_layer(transport_layer)
            .with_user_agent(&config.user_agent);

        let endpoint = endpoint_builder.build();

        // 启动端点服务
        let endpoint_for_serve = endpoint.inner.clone();
        tokio::spawn(async move {
            endpoint_for_serve.serve().await.ok();
        });

        // 创建对话层
        let dialog_layer = Arc::new(DialogLayer::new(endpoint.inner.clone()));

        // 启动传入请求处理
        Self::start_incoming_handler(
            endpoint.incoming_transactions()?,
            dialog_layer.clone(),
            cancel_token.clone(),
        );

        Ok(Self {
            config,
            endpoint,
            dialog_layer,
            cancel_token,
        })
    }

    /// 启动传入请求处理器
    fn start_incoming_handler(
        mut incoming: rsipstack::transaction::TransactionReceiver,
        dialog_layer: Arc<DialogLayer>,
        cancel_token: CancellationToken,
    ) {
        tokio::spawn(async move {
            while let Some(mut transaction) = tokio::select! {
                tx = incoming.recv() => tx,
                _ = cancel_token.cancelled() => None,
            } {
                let method = transaction.original.method;
                debug!("收到传入请求: {}", method);

                if let Some(mut dialog) = dialog_layer.match_dialog(&transaction.original) {
                    tokio::spawn(async move {
                        if let Err(e) = dialog.handle(&mut transaction).await {
                            error!("处理 {} 请求失败: {}", method, e);
                        }
                    });
                } else {
                    warn!("未找到匹配的对话: {}", method);
                }
            }
        });
    }

    /// 执行注册
    pub async fn register(&self) -> CallResult<Response> {
        info!("正在注册到 SIP 服务器...");

        let actual_local_addr = self
            .endpoint
            .get_addrs()
            .first()
            .ok_or(CallError::NotInitialized)?
            .addr
            .clone();

        info!("本地绑定的实际地址: {}", actual_local_addr);

        // 构造注册URI（从 config.server 复制并移除 transport 参数）
        let mut register_uri = self.config.server.clone();

        // 移除 transport 参数（如果有）registrar 不需要 transport 参数
        register_uri
            .params
            .retain(|p| !matches!(p, rsip::Param::Transport(_)));

        info!("Register URI: {}", register_uri);

        // 创建认证凭证
        let credential = Credential {
            username: self.config.username.clone(),
            password: self.config.password.clone(),
            realm: None, // 将从 401 响应自动提取
        };

        // 创建 Registration 实例（全局 route_set 已在 Endpoint 层面配置）
        let mut registration = Registration::new(self.endpoint.inner.clone(), Some(credential));

        registration.call_id = Uuid::new_v4().to_string().into();
        // 执行注册
        let response = registration.register(register_uri.clone(), Some(3600)).await?;
        
        if response.status_code == rsip::StatusCode::OK {
            info!("✔ 注册成功,响应状态: {}", response.status_code);
        } else {
            warn!("注册响应: {}", response.status_code);
            
            // 根据状态码返回适当的错误
            match response.status_code {
                rsip::StatusCode::Unauthorized => {
                    return Err(CallError::AuthenticationFailed { 
                        reason: "认证失败".to_string() 
                    });
                }
                rsip::StatusCode::NotFound => {
                    return Err(CallError::InvalidTarget { 
                        target: "注册目标未找到".to_string() 
                    });
                }
                rsip::StatusCode::ServerInternalError |
                rsip::StatusCode::ServiceUnavailable => {
                    let port = register_uri.host_with_port.port.unwrap_or_else(|| 5060.into());
                    return Err(CallError::NetworkConnection { 
                        host: register_uri.host_with_port.to_string(),
                        port: port.into()
                    });
                }
                _ => {
                    return Err(CallError::Other(
                        format!("注册失败: {} {}", response.status_code, 
                                String::from_utf8_lossy(&response.body)).into()
                    ));
                }
            }
        }
        
        Ok(response)
    }

    /// 发起呼叫
    pub async fn make_call(&self, target: &str,sdp_offer: &str) -> CallResult<(ClientInviteDialog, Option<Response>)> {
        info!("📞发起呼叫到: {}", target);

        let actual_local_addr = self
            .endpoint
            .get_addrs()
            .first()
            .ok_or(CallError::NotInitialized)?
            .addr
            .clone();

        let contact_uri_str = format!("sip:{}@{}", self.config.username, actual_local_addr);

        // 构造 From/To URI（使用服务器URI的域名部分）
        let server_domain = self.config.server.host_with_port.to_string();

        let from_uri = format!("sip:{}@{}", self.config.username, server_domain);
        let to_uri = if target.contains('@') {
            format!("sip:{}", target)
        } else {
            format!("sip:{}@{}", target, server_domain)
        };

        info!("Call信息 源：{} -> 目标：{}", from_uri, to_uri);


        // 生成呼叫 Call-ID（直接使用 UUID 字符串）
        let call_id_string = Uuid::new_v4().to_string();
        info!("生成呼叫 Call-ID: {}", call_id_string);

        // 创建认证凭证
        let credential = Credential {
            username: self.config.username.clone(),
            password: self.config.password.clone(),
            realm: None, // 将从 401/407 响应自动提取
        };

        // 全局 route_set 已在 Endpoint 层面配置，INVITE 会自动使用
        let invite_opt = InviteOption {
            caller: from_uri.as_str().try_into()?,
            callee: to_uri.as_str().try_into()?,
            contact: contact_uri_str.as_str().try_into()?,
            credential: Some(credential),
            caller_display_name: None,
            caller_params: vec![],
            destination: None, // 让 rsipstack 自动从 Route header 解析
            content_type: Some("application/sdp".to_string()),
            offer: Some(sdp_offer.as_bytes().to_vec()),
            headers: None, // 不需要手动添加，rsipstack 自动处理
            support_prack: false,
            call_id: Some(call_id_string),
        };

        // 创建状态通道
        let (state_sender, _state_receiver) = self.dialog_layer.new_dialog_state_channel();

        // 发送 INVITE
        let (dialog, response) = self
            .dialog_layer
            .do_invite(invite_opt, state_sender)
            .await?;

        let dialog_id = dialog.id();
        info!(
            "✅ INVITE 请求已发送，Dialog -> Call-ID: {} From-Tag: {} To-Tag: {}",
            dialog_id.call_id, dialog_id.local_tag, dialog_id.remote_tag
        );

        // if let Some(resp) = response {
        //     info!("响应状态: {}", resp.status_code());
        //
        //     // 处理 SDP Answer
        //     let body = resp.body();
        //     if !body.is_empty() {
        //         let sdp_answer = String::from_utf8_lossy(body);
        //         debug!("SDP Answer: {}", sdp_answer);
        //     }
        // }

        Ok((dialog, response))
    }

    /// 注销
    pub async fn unregister(&self) -> CallResult<Response> {
        info!("正在从SIP服务器注销...");
        
        let _actual_local_addr = self
            .endpoint
            .get_addrs()
            .first()
            .ok_or(CallError::NotInitialized)?
            .addr
            .clone();
        
        // 构造注册URI（从 config.server 复制并移除 transport 参数）
        let mut register_uri = self.config.server.clone();
        
        // 移除 transport 参数（如果有）registrar 不需要 transport 参数
        register_uri
            .params
            .retain(|p| !matches!(p, rsip::Param::Transport(_)));
        
        info!("Unregister URI: {}", register_uri);
        
        // 创建认证凭证
        let credential = Credential {
            username: self.config.username.clone(),
            password: self.config.password.clone(),
            realm: None, // 将从 401 响应自动提取
        };
        
        // 创建 Registration 实例（全局 route_set 已在 Endpoint 层面配置）
        let mut registration = Registration::new(self.endpoint.inner.clone(), Some(credential));
        
        registration.call_id = Uuid::new_v4().to_string().into();
        
        // 执行注销（expires=0表示注销）
        let response = registration.register(register_uri, Some(0)).await?;
        
        if response.status_code == rsip::StatusCode::OK {
            info!("✔ 注销成功,响应状态: {}", response.status_code);
        } else {
            warn!("注销响应: {}", response.status_code);
        }
        
        Ok(response)
    }

    /// 关闭客户端
    pub async fn shutdown(&self) {
        self.cancel_token.cancel();
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}
