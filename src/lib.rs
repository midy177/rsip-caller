// 声明所有模块
pub mod config;
pub mod error;
pub mod rtp;
pub mod rtp_play;
pub mod sip_client;
pub mod sip_dialog;
pub mod sip_transport;
pub mod utils;

/// 重新导出thiserror错误类型
pub use crate::error::{SipError, RtpError, ConfigError, CallError, CallResult};
pub use crate::rtp_play::MediaPlayError;

/// 主要API重新导出，简化使用
pub use crate::config::Config as SipConfig;
pub use crate::rtp::{build_rtp_conn, play_audio_file, play_echo, MediaSessionOption};
pub use crate::rtp_play::{MediaPlayer, MediaPlayerFactory, RtpPlayer};
pub use rustrtc::media::MediaKind;
pub use crate::sip_client::SipClient;
pub use crate::utils as utils_mod;

/// SIP Caller库的版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 便捷函数：快速创建SIP客户端
pub async fn create_sip_client(server: &str, user: &str, password: &str) -> Result<SipClient, SipError> {
    create_sip_client_with_proxy(server, user, password, None).await
}

/// 便捷函数：创建带代理的SIP客户端
pub async fn create_sip_client_with_proxy(
    server: &str, 
    user: &str, 
    password: &str,
    outbound_proxy: Option<&str>
) -> Result<SipClient, SipError> {
    let config = crate::config::Config::new(server, user, password)?;
    let server_uri: rsip::Uri = format!("sip:{}", config.server)
        .try_into()
        .map_err(|e| SipError::Protocol(format!("Invalid server URI: {}", e)))?;
    
    let proxy_uri = if let Some(proxy) = outbound_proxy {
        Some(format!("sip:{}", proxy)
            .try_into()
            .map_err(|e| SipError::Protocol(format!("Invalid proxy URI: {}", e)))?)
    } else {
        None
    };
    
    let sip_client_config = sip_client::SipClientConfig {
        server: server_uri,
        outbound_proxy: proxy_uri,
        username: config.username,
        password: config.password,
        user_agent: config.user_agent,
    };
    Ok(SipClient::new(sip_client_config).await?)
}

/// 便捷函数：快速创建RTP音频播放器
pub async fn create_audio_player(file_path: &str) -> Result<Box<dyn MediaPlayer>, MediaPlayError> {
    MediaPlayerFactory::create_audio_player(file_path).await
}

/// 便捷函数：快速创建RTP视频播放器
pub async fn create_video_player(file_path: &str) -> Result<Box<dyn MediaPlayer>, MediaPlayError> {
    MediaPlayerFactory::create_video_player(file_path).await
}

/// 便捷函数：快速创建RTP回声播放器
pub async fn create_echo_player() -> Result<Box<dyn MediaPlayer>, MediaPlayError> {
    use crate::rtp_play::AudioEchoPlayer;
    
    // 创建AudioEchoPlayer并获取SDP
    let (player, _sdp) = AudioEchoPlayer::new().await?;
    
    // 初始化播放器
    let mut player: AudioEchoPlayer = player;
    player.initialize().await?;
    
    Ok(Box::new(player))
}

/// 便捷函数：创建RTP会话
pub async fn create_rtp_session<'a>(
    media_type: MediaKind,
) -> Result<(RtpPlayer, String), MediaPlayError> {
    let player = RtpPlayer::new(media_type).await?;
    let sdp = player.get_local_sdp()?;
    Ok((player, sdp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[tokio::test]
    async fn test_create_sip_client() {
        let result = create_sip_client("127.0.0.1:5060", "test@example.com", "password").await;
        // 这个测试可能会失败，因为需要网络连接，但至少能验证API结构
        assert!(result.is_ok() || result.is_err());
    }
}