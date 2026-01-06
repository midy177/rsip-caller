use clap::Parser;
/// SIP Caller 主程序（使用 rsipstack）
///
/// 演示如何使用 rsipstack 进行注册和呼叫
mod config;
mod dialog;
pub mod registration;
mod rtp;
mod transport;
mod utils;

use config::Protocol;
use dialog::process_dialog;
use registration::{RegistrarFactory, RegistrarFactoryConfig, RegistrationConfig};
use transport::{create_transport_connection, extract_peer_rtp_addr};
use utils::get_first_non_loopback_interface;

use rand::Rng;
use rsipstack::{
    dialog::{
        authenticate::Credential, dialog_layer::DialogLayer, invitation::InviteOption,
    },
    transport::TransportLayer,
    EndpointBuilder,
};
use rtp::{build_rtp_conn, MediaSessionOption};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

/// SIP Caller - 基于 Rust 的 SIP 客户端
#[derive(Parser, Debug)]
#[command(name = "sip-caller")]
#[command(author = "SIP Caller Team")]
#[command(version = "0.2.0")]
#[command(about = "SIP 客户端，支持注册和呼叫功能", long_about = None)]
struct Args {
    /// SIP 服务器地址（例如：127.0.0.1:5060）
    #[arg(short, long, default_value = "xfc:5060")]
    server: String,

    /// 传输协议类型 (udp, tcp, ws, wss)
    #[arg(long, default_value = "udp")]
    protocol: Protocol,

    /// Outbound 代理服务器地址（可选，例如：proxy.example.com:5060）
    #[arg(long)]
    outbound_proxy: Option<String>,

    /// SIP 用户 ID（例如：alice@example.com）
    #[arg(short, long, default_value = "1001")]
    user: String,

    /// SIP 密码
    #[arg(short, long, default_value = "admin")]
    password: String,

    /// 呼叫目标（例如：bob@example.com）
    #[arg(short, long, default_value = "1000")]
    target: String,

    /// 本地 SIP 端口
    #[arg(long, default_value = "0")]
    local_port: u16,

    /// 优先使用 IPv6（找不到时自动回退到 IPv4）
    #[arg(long, default_value = "false")]
    ipv6: bool,

    /// RTP 起始端口
    #[arg(long, default_value = "20000")]
    rtp_start_port: u16,

    /// 是否启用回声模式
    #[arg(long, default_value = "true")]
    echo_mode: bool,

    /// User-Agent 标识
    #[arg(long, default_value = "RSipCaller/0.2.0")]
    user_agent: String,

    /// 日志级别 (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 解析命令行参数
    let args = Args::parse();

    // 初始化日志系统
    let log_level = match args.log_level.to_lowercase().as_str() {
        "trace" => tracing::Level::TRACE,
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => {
            eprintln!("无效的日志级别 '{}', 使用默认值 'info'", args.log_level);
            tracing::Level::INFO
        }
    };

    tracing_subscriber::fmt().with_max_level(log_level).init();

    info!(
        "SIP Caller 启动 - 服务器: {}, 协议: {}, 代理: {}, 用户: {}, 目标: {}, IPv6: {}, RTP端口: {}, User-Agent: {}",
        args.server,
        args.protocol,
        args.outbound_proxy.as_deref().unwrap_or("无"),
        args.user,
        args.target,
        args.ipv6,
        args.rtp_start_port,
        args.user_agent
    );

    let cancel_token = CancellationToken::new();

    // 创建传输层
    let transport_layer = TransportLayer::new(cancel_token.clone());

    // 获取本地 IP
    let local_ip = get_first_non_loopback_interface(args.ipv6)?;
    info!(
        "检测到本地出口IP: {} ({})",
        local_ip,
        if local_ip.is_ipv6() { "IPv6" } else { "IPv4" }
    );

    // 确定实际连接的服务器地址（如果有 Outbound 代理则连接到代理）
    let connection_target = args.outbound_proxy.as_ref().unwrap_or(&args.server);
    if args.outbound_proxy.is_some() {
        info!("使用 Outbound 代理: {}", connection_target);
    }

    // 根据协议类型创建传输连接
    let local_addr = format!("{}:{}", local_ip, args.local_port).parse()?;
    let connection = create_transport_connection(
        args.protocol,
        local_addr,
        connection_target,
        cancel_token.clone(),
    )
    .await?;

    transport_layer.add_transport(connection);

    // 创建端点
    let endpoint = EndpointBuilder::new()
        .with_cancel_token(cancel_token.clone())
        .with_transport_layer(transport_layer)
        .with_user_agent(&args.user_agent)
        .build();

    // 启动端点服务（必须！用于接收网络消息）
    let endpoint_for_serve = endpoint.inner.clone();
    tokio::spawn(async move {
        endpoint_for_serve.serve().await.ok();
    });

    // 获取传入事务接收器用于处理服务端请求（如 INVITE 等）
    let mut incoming = endpoint.incoming_transactions()?;

    // 创建对话层（需要在处理传入请求之前创建）
    let dialog_layer = Arc::new(DialogLayer::new(endpoint.inner.clone()));
    let dialog_layer_for_incoming = dialog_layer.clone();

    // 启动后台任务处理传入的请求
    let incoming_cancel = cancel_token.clone();
    tokio::spawn(async move {
        while let Some(mut transaction) = tokio::select! {
            tx = incoming.recv() => tx,
            _ = incoming_cancel.cancelled() => None,
        } {
            let method = transaction.original.method.clone();
            debug!(
                "收到传入请求-> method: {} uri: {} version: {} headers: {} body: {:?}",
                method,
                transaction.original.uri.clone(),
                transaction.original.version.clone(),
                transaction.original.headers.clone(),
                transaction.original.body.clone()
            );

            // 尝试匹配到现有对话
            if let Some(mut dialog) = dialog_layer_for_incoming.match_dialog(&transaction.original)
            {
                // 让对话处理这个事务（会自动发送响应）
                tokio::spawn(async move {
                    if let Err(e) = dialog.handle(&mut transaction).await {
                        error!("处理 {} 请求失败: {}", method, e);
                    }
                    Ok::<_, rsipstack::Error>(())
                });
            } else {
                // 没有匹配的对话，发送 481 Call/Transaction Does Not Exist
                warn!("未找到匹配的对话: {}", method);
            }
        }
    });

    // 获取实际绑定的本地地址
    let actual_local_addr = endpoint
        .get_addrs()
        .first()
        .ok_or("未找到地址")?
        .addr
        .clone();

    info!("本地绑定的实际地址: {}", actual_local_addr);

    // 提取域名和端口
    let server_parts: Vec<&str> = args.server.split(':').collect();
    let server_host = server_parts[0];
    let server_port = server_parts
        .get(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(5060u16);

    // 构造 Registration URI
    // 当使用 Outbound 代理时，如果 server_host 不是有效的IP/域名（如租户ID），
    // 使用代理地址作为 Register URI，租户信息保留在 domain_for_from_to 中
    let is_tenant_id = args.outbound_proxy.is_some()
        && !server_host.contains('.')  // 不包含点（不是域名或IP）
        && !server_host.parse::<std::net::IpAddr>().is_ok();  // 不是有效IP

    let (register_uri_str, domain_for_from_to) = if is_tenant_id {
        // 租户ID模式：使用租户地址作为 Register URI（匹配话机行为）
        info!("检测到租户ID: {}, 使用 Outbound 代理模式", server_host);
        (
            format!("sip:{}:{}", server_host, server_port),  // 使用租户地址！
            server_host.to_string(),  // 保留租户ID用于From/To
        )
    } else if args.outbound_proxy.is_some() {
        // 有代理但 server 是正常域名/IP
        (
            format!("sip:{}:{}", server_host, server_port),
            server_host.to_string(),
        )
    } else {
        // 无代理模式
        let uri = format!("sip:{}:{}", server_host, server_port);
        (uri.clone(), server_host.to_string())
    };

    let server_uri_parsed: rsip::Uri = register_uri_str.as_str().try_into()?;
    let contact_uri_str = format!("sip:{}@{}", args.user, actual_local_addr);

    info!(
        "Register URI: {}, Contact URI: {}, 租户域名: {}",
        register_uri_str, contact_uri_str, domain_for_from_to
    );

    if is_tenant_id {
        info!(
            "多租户模式 -> 物理连接: {}, Register URI: {}, From/To域名: {}",
            connection_target, register_uri_str, domain_for_from_to
        );
    }

    // 使用自定义的 make_call_id 函数（基于 UUID）
    let register_call_id = utils::make_uuid_call_id();
    info!("生成注册 Call-ID: {}", register_call_id.to_string());

    // 创建注册配置
    let mut registration_config = RegistrationConfig::new(args.user.clone(), args.password.clone())
        .with_call_id(register_call_id.clone())
        .with_contact_uri(contact_uri_str.clone())
        .with_expires(3600);

    // 如果是租户ID，设置 realm
    if is_tenant_id {
        registration_config = registration_config.with_realm(server_host.to_string());
    }

    // 注意: EndpointBuilder.with_user_agent() 会为所有请求设置 User-Agent
    // 包括 REGISTER 和 INVITE 请求

    info!("正在注册到 SIP 服务器...");

    // 使用工厂模式创建注册器
    let registrar_type = RegistrarFactory::auto_detect(server_host, args.outbound_proxy.is_some());

    let factory_config = match registrar_type {
        registration::RegistrarType::OutboundProxy => {
            info!("使用 Outbound 代理注册模式");
            RegistrarFactoryConfig::outbound_proxy(
                registration_config,
                domain_for_from_to.clone(),
                connection_target.to_string(),
            )
        }
        registration::RegistrarType::Standard => {
            info!("使用标准注册模式");
            RegistrarFactoryConfig::standard(registration_config)
        }
    };

    let mut registrar = RegistrarFactory::create(endpoint.inner.clone(), factory_config);

    // 执行注册
    match registrar.register(server_uri_parsed.clone(), Some(3600)).await {
        Ok(response) => {
            if response.status_code == rsip::StatusCode::OK {
                info!("✔ 注册成功,响应状态: {}", response.status_code);

                // 显示公共地址（如果有）
                if let Some(public_addr) = registrar.public_address() {
                    info!("检测到公共地址: {}", public_addr);
                }
            } else {
                warn!("注册响应: {}", response.status_code);
            }
        }
        Err(e) => {
            error!("❌ 注册失败: {}", e);
            return Err(format!("注册失败: {}", e).into());
        }
    }

    // 创建认证凭证（用于后续的 INVITE 请求）
    let credential = Credential {
        username: args.user.clone(),
        password: args.password.clone(),
        realm: if is_tenant_id {
            Some(server_host.to_string())
        } else {
            None
        },
    };

    // 等待一段时间确保注册完成
    // tokio::time::sleep(Duration::from_secs(1)).await;

    // 发起呼叫
    info!("📞发起呼叫到: {}", args.target);

    let from_uri = format!("sip:{}@{}", args.user, domain_for_from_to);
    let to_uri = if args.target.contains('@') {
        format!("sip:{}", args.target)
    } else {
        format!("sip:{}@{}", args.target, domain_for_from_to)
    };

    info!("Call信息 源：{} -> 目标：{}", from_uri, to_uri);

    // 准备 RTP 会话
    let rtp_cancel = cancel_token.child_token();
    let media_opt = MediaSessionOption {
        rtp_start_port: args.rtp_start_port,
        external_ip: None,
        cancel_token: rtp_cancel.clone(),
        echo_mode: args.echo_mode,
    };

    // 生成随机 SSRC
    let ssrc = rand::rng().random::<u32>();
    let payload_type = 0u8; // PCMU

    // 创建 RTP 连接
    let (rtp_conn, sdp_offer) = build_rtp_conn(local_ip, &media_opt, ssrc, payload_type).await?;
    debug!("SDP Offer:{}", sdp_offer);

    // 使用自定义的 make_call_id 函数（基于 UUID）
    let call_id = utils::make_uuid_call_id();
    info!("生成呼叫 Call-ID: {}", call_id.to_string());

    // 在多租户模式下，destination 需要设置为代理地址
    let destination = if is_tenant_id {
        info!("多租户模式：INVITE 将发送到代理 {}", connection_target);
        // 将代理地址转换为 SipAddr
        let proxy_host_port: rsip::HostWithPort = connection_target.as_str().try_into()?;
        let sip_addr = rsipstack::transport::SipAddr::new(
            args.protocol.to_rsip_transport(),
            proxy_host_port,
        );
        Some(sip_addr)
    } else {
        None
    };

    let invite_opt = InviteOption {
        caller: from_uri.as_str().try_into()?,
        callee: to_uri.as_str().try_into()?,
        contact: contact_uri_str.as_str().try_into()?,
        credential: Some(credential),
        caller_display_name: None,
        caller_params: vec![],
        destination,  // 多租户模式下使用代理地址
        content_type: Some("application/sdp".to_string()),
        offer: Some(sdp_offer.as_bytes().to_vec()),
        headers: None, // User-Agent 已在 Endpoint 层面设置
        support_prack: false,
        call_id: Some(call_id.to_string()),
    };

    // 创建状态通道
    let (state_sender, state_receiver) = dialog_layer.new_dialog_state_channel();

    match dialog_layer.do_invite(invite_opt, state_sender).await {
        Ok((dialog, response)) => {
            let dialog_id = dialog.id();
            info!(
                "✅ INVITE 请求已发送，Dialog -> Call-ID: {} From-Tag: {} To-Tag: {}",
                dialog_id.call_id, dialog_id.from_tag, dialog_id.to_tag
            );

            if let Some(resp) = response {
                info!("响应状态: {}", resp.status_code());

                // 提取 SDP Answer
                let body = resp.body();
                if !body.is_empty() {
                    let sdp_answer = String::from_utf8_lossy(body);
                    debug!("SDP Answer: {}", sdp_answer);

                    // 提取对端 RTP 地址
                    if let Some(peer_addr) = extract_peer_rtp_addr(&sdp_answer) {
                        info!("✓ 对端 RTP 地址: {}", peer_addr);

                        // 启动对话状态监控
                        let dialog_clone = Arc::new(dialog.clone());
                        let rtp_cancel_clone = rtp_cancel.clone();
                        tokio::spawn(async move {
                            process_dialog(dialog_clone, state_receiver, rtp_cancel_clone).await;
                        });

                        // 启动 RTP 会话（回声模式）
                        info!(
                            "🔊 启动回声模式: {}",
                            if args.echo_mode {
                                "已启用"
                            } else {
                                "已禁用"
                            }
                        );
                        if args.echo_mode {
                            let rtp_cancel_clone = rtp_cancel.clone();
                            let peer_addr_clone = peer_addr.clone();
                            tokio::spawn(async move {
                                if let Err(e) = rtp::play_echo(
                                    rtp_conn,
                                    rtp_cancel_clone,
                                    peer_addr_clone,
                                    ssrc,
                                )
                                .await
                                {
                                    error!("RTP 回声播放失败: {}", e);
                                }
                            });
                        }

                        // 等待用户手动挂断
                        info!("📞 通话中，按 Ctrl+C 手动挂断");
                        match tokio::signal::ctrl_c().await {
                            Ok(()) => {}
                            Err(err) => {
                                error!("无法监听 Ctrl+C 信号: {}", err);
                            }
                        }

                        // 挂断呼叫
                        match dialog.bye().await {
                            Ok(_) => {
                                info!("✅ 通话结束");
                            }
                            Err(e) => {
                                warn!("发送 BYE 失败: {}", e);
                            }
                        }

                        // 取消 RTP 会话
                        rtp_cancel.cancel();
                    } else {
                        error!("无法从 SDP Answer 中提取对端 RTP 地址");
                    }
                }
            }
        }
        Err(e) => {
            error!("呼叫失败: {}", e);
            return Err(format!("呼叫失败: {}", e).into());
        }
    }

    cancel_token.cancel();

    // 等待一小段时间让所有任务完成
    tokio::time::sleep(Duration::from_millis(500)).await;
    Ok(())
}
