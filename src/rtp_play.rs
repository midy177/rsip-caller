use async_trait::async_trait;
use rustrtc::media::{
    MediaError, MediaKind, MediaSample,
    MediaStreamTrack,
};
use rustrtc::{
    PeerConnection, RtcConfiguration, SdpType, SessionDescription, TransportMode,
    RtpCodecParameters,
};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

/// Custom error type for media playback operations
#[derive(Debug, Error)]
pub enum MediaPlayError {
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Echo not initialized")]
    EchoNotInitialized,
    
    #[error("SDP error: {0}")]
    Sdp(String),
    
    #[error("RTP error: {0}")]
    Rtp(String),
}

impl From<MediaError> for MediaPlayError {
    fn from(error: MediaError) -> Self {
        match error {
            MediaError::Closed => MediaPlayError::Sdp("Connection closed".to_string()),
            MediaError::EndOfStream => MediaPlayError::Sdp("End of stream".to_string()),
            MediaError::Lagged => MediaPlayError::Rtp("Lagged".to_string()),
            MediaError::KindMismatch { expected, actual } => {
                MediaPlayError::Sdp(format!("Kind mismatch: expected {:?}, got {:?}", expected, actual))
            }
        }
    }
}

/// 媒体播放器工厂，用于创建不同类型的媒体播放器
pub struct MediaPlayerFactory;

impl MediaPlayerFactory {
    /// 创建音频播放器
    pub async fn create_audio_player(file_path: &str) -> Result<Box<dyn MediaPlayer>, MediaPlayError> {
        let path = PathBuf::from(file_path);
        Self::validate_file_exists(&path, file_path)?;
        
        let ext = Self::get_file_extension(&path);
        match ext.as_str() {
            "wav" => {
                let player = RtpPlayer::new(MediaKind::Audio).await?;
                Ok(Box::new(player))
            }
            _ => Err(MediaPlayError::UnsupportedFormat("不支持的音频格式".to_string())),
        }
    }
    
    /// 创建视频播放器
    pub async fn create_video_player(file_path: &str) -> Result<Box<dyn MediaPlayer>, MediaPlayError> {
        let path = PathBuf::from(file_path);
        Self::validate_file_exists(&path, file_path)?;
        
        let ext = Self::get_file_extension(&path);
        match ext.as_str() {
            "ivf" => {
                let player = RtpPlayer::new(MediaKind::Video).await?;
                Ok(Box::new(player))
            }
            _ => Err(MediaPlayError::UnsupportedFormat("不支持的视频格式".to_string())),
        }
    }
    
    /// 创建音频回声播放器
    pub async fn create_echo_player() -> Result<Box<dyn MediaPlayer>, MediaPlayError> {
        let (player, _sdp) = AudioEchoPlayer::new().await?;
        Ok(Box::new(player))
    }
    
    // 私有辅助方法
    fn validate_file_exists(path: &PathBuf, _file_path: &str) -> Result<(), MediaPlayError> {
        if !path.exists() {
            return Err(MediaPlayError::FileNotFound("File not found".to_string()));
        }
        Ok(())
    }
    
    fn get_file_extension(path: &PathBuf) -> String {
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase()
    }
}

/// 媒体播放器通用接口
#[async_trait]
pub trait MediaPlayer {
    /// 获取媒体类型
    fn media_kind(&self) -> MediaKind;
    
    /// 获取RTP载荷类型
    fn payload_type(&self) -> u8;
    
    /// 获取时钟频率
    fn clock_rate(&self) -> u32;
    
    /// 播放媒体到指定的远程地址
    async fn play_to_remote(&mut self, peer_connection: Arc<PeerConnection>) -> Result<(), MediaPlayError>;
    
    /// 启动回声模式
    async fn start_echo(&mut self) -> Result<(), MediaPlayError> {
        Err(MediaPlayError::Sdp("此播放器不支持回声模式".to_string()))
    }
    
    /// 获取本地SDP
    async fn get_local_sdp(&self) -> Result<String, MediaPlayError> {
        Err(MediaPlayError::Sdp("此播放器不支持获取SDP".to_string()))
    }
}

/// RTP播放器，用于生成SDP并播放媒体
pub struct RtpPlayer {
    peer_connection: Arc<PeerConnection>,
    running: Option<Arc<std::sync::atomic::AtomicBool>>,
    is_active: bool,
}

impl RtpPlayer {
    /// 创建新的RTP播放器
    pub async fn new(media_type: MediaKind) -> Result<Self, MediaPlayError> {
        let config = Self::create_rtc_config();
        let pc = Arc::new(PeerConnection::new(config));
        
        // 创建媒体轨道
        let (_sample_source, track, _) = rustrtc::media::sample_track(media_type.clone(), 100);
        
        // 设置编解码器参数
        let params = Self::create_codec_params(media_type.clone());
        
        pc.add_track(track, params)
            .map_err(|e| MediaPlayError::Rtp(format!("添加轨道失败: {}", e)))?;
        
        // 创建并设置offer
        let local_sdp = pc.create_offer()
            .await
            .map_err(|e| MediaPlayError::Sdp(format!("创建offer失败: {}", e)))?;
            
        pc.set_local_description(local_sdp.clone())
            .map_err(|e| MediaPlayError::Sdp(format!("设置本地描述失败: {}", e)))?;
        
        // 等待收集完成
        let pc_clone = pc.clone();
        tokio::spawn(async move {
            let _ = pc_clone.wait_for_gathering_complete().await;
        });
        
        Ok(Self {
            peer_connection: pc,
            running: None,
            is_active: false,
        })
    }
    
    fn create_codec_params(media_type: MediaKind) -> RtpCodecParameters {
        match media_type {
            MediaKind::Audio => RtpCodecParameters {
                payload_type: 0, // PCMU
                clock_rate: 8000,
                channels: 1,
            },
            MediaKind::Video => RtpCodecParameters {
                payload_type: 96, // VP8
                clock_rate: 90000,
                channels: 0,
            },
        }
    }
    
    /// 获取本地SDP
    pub fn get_local_sdp(&self) -> Result<String, MediaPlayError> {
        // 从 PeerConnection 获取当前本地描述
        let local_desc = self.peer_connection.local_description()
            .ok_or_else(|| MediaPlayError::Sdp("本地描述未设置".to_string()))?;
            
        Ok(local_desc.to_sdp_string())
    }
    
    /// 设置远程SDP并开始播放
    pub async fn set_remote_sdp_and_play(
        &mut self,
        remote_sdp: &str,
        mut media_player: Box<dyn MediaPlayer>,
    ) -> Result<(), MediaPlayError> {
        // 解析并设置远程SDP
        let remote_sdp = SessionDescription::parse(SdpType::Answer, remote_sdp)
            .map_err(|e| MediaPlayError::Sdp(format!("解析远程SDP失败: {}", e)))?;
        
        self.peer_connection.set_remote_description(remote_sdp)
            .await
            .map_err(|e| MediaPlayError::Sdp(format!("设置远程描述失败: {}", e)))?;
        
        // 开始播放媒体
        media_player.play_to_remote(self.peer_connection.clone()).await?;
        
        Ok(())
    }
    
    /// 启动音频回声
    pub async fn start_audio_echo(&mut self) -> Result<(), MediaPlayError> {
        info!("启动音频回声功能");
        
        // 检查是否已经运行
        if self.running.is_some() {
            warn!("音频回声处理器已在运行");
            return Ok(());
        }
        
        // 创建运行标志
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
        self.running = Some(running.clone());
        
        let transceivers = self.peer_connection.get_transceivers();
        
        for transceiver in transceivers {
            if transceiver.kind() != rustrtc::MediaKind::Audio {
                continue;
            }
            
            // 设置为双向通信
            transceiver.set_direction(rustrtc::TransceiverDirection::SendRecv);
            
            // 获取接收器
            let receiver = transceiver.receiver();
            let Some(receiver) = receiver else {
                warn!("音频收发器缺少接收器");
                continue;
            };
            
            let incoming_track = receiver.track();
            let (sample_source, outgoing_track, _) = rustrtc::media::sample_track(MediaKind::Audio, 100);
            
            // 创建发送器
            let ssrc = 5000 + transceiver.id() as u32;
            let sender = rustrtc::peer_connection::RtpSender::builder(outgoing_track, ssrc)
                .stream_id("echo-stream".to_string())
                .params(rustrtc::RtpCodecParameters {
                    payload_type: 0, // PCMU
                    clock_rate: 8000,
                    channels: 1,
                })
                .build();
                
            // 订阅RTCP以处理PLI/FIR请求
            let mut rtcp_rx = sender.subscribe_rtcp();
            let incoming_track_clone = incoming_track.clone();
            tokio::spawn(async move {
                while let Ok(packet) = rtcp_rx.recv().await {
                    match packet {
                        rustrtc::rtp::RtcpPacket::PictureLossIndication(_)
                        | rustrtc::rtp::RtcpPacket::FullIntraRequest(_) => {
                            if let Err(e) = incoming_track_clone.request_key_frame().await {
                                warn!("请求关键帧失败: {}", e);
                            } else {
                                info!("转发PLI/FIR到入站轨道");
                            }
                        }
                        _ => {}
                    }
                }
            });
            
            transceiver.set_sender(Some(sender));
            
            // 启动回声循环
            let _pc_clone = self.peer_connection.clone();
            tokio::spawn(async move {
                info!("音频回声循环已启动");
                
                loop {
                    match incoming_track.recv().await {
                        Ok(sample) => {
                            // 检查样本是否为空
                            let is_empty = match &sample {
                                MediaSample::Audio(f) => f.data.is_empty(),
                                MediaSample::Video(_) => false,
                            };
                            
                            if is_empty {
                                continue;
                            }
                            
                            // 直接转发收到的音频样本
                            if let Err(e) = sample_source.send(sample).await {
                                warn!("音频回声转发失败: {}", e);
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("音频入站轨道结束: {}", e);
                            break;
                        }
                    }
                }
                
                info!("音频回声循环已停止");
            });
        }
        
        self.is_active = true;
        info!("音频回声处理器已启动");
        Ok(())
    }
    
    /// 停止回声处理
    pub fn stop_echo(&mut self) {
        if !self.is_active {
            warn!("回声处理器未运行");
            return;
        }
        
        self.is_active = false;
        
        // 关闭PeerConnection
        info!("正在关闭音频回声处理器");
        // PeerConnection没有直接的关闭方法，但我们可以在析构时处理
    }
    
    /// 设置远程SDP
    pub async fn set_remote_sdp(&mut self, remote_sdp: &str) -> Result<(), MediaPlayError> {
        self.ensure_initialized()?;
        
        let remote_sdp = SessionDescription::parse(SdpType::Answer, remote_sdp)
            .map_err(|e| MediaPlayError::Sdp(format!("解析远程SDP失败: {}", e)))?;
        
        let pc = &self.peer_connection;
        pc.set_remote_description(remote_sdp)
            .await
            .map_err(|e| MediaPlayError::Sdp(format!("设置远程描述失败: {}", e)))?;
        
        info!("远程SDP设置成功");
        
        // 启动回声
        self.start_audio_echo().await?;
        Ok(())
    }
    
    /// 检查是否已初始化
    pub fn is_initialized(&self) -> bool {
        true // RtpPlayer is always initialized after new()
    }
    
    /// 检查回声是否正在运行
    pub fn is_echo_running(&self) -> bool {
        self.is_active
    }
    
    // 私有辅助方法
    fn create_rtc_config() -> RtcConfiguration {
        let mut config = RtcConfiguration::default();
        config.transport_mode = TransportMode::Rtp;
        config
    }
    
    fn ensure_initialized(&self) -> Result<(), MediaPlayError> {
        if !self.is_initialized() {
            return Err(MediaPlayError::EchoNotInitialized);
        }
        Ok(())
    }
}

#[async_trait]
impl MediaPlayer for RtpPlayer {
    fn media_kind(&self) -> MediaKind {
        MediaKind::Audio
    }
    
    fn payload_type(&self) -> u8 {
        0 // PCMU
    }
    
    fn clock_rate(&self) -> u32 {
        8000
    }
    
    async fn play_to_remote(&mut self, _peer_connection: Arc<PeerConnection>) -> Result<(), MediaPlayError> {
        Err(MediaPlayError::Sdp("RtpPlayer不支持此操作".to_string()))
    }
    
    async fn start_echo(&mut self) -> Result<(), MediaPlayError> {
        self.start_audio_echo().await
    }
    
    async fn get_local_sdp(&self) -> Result<String, MediaPlayError> {
        self.get_local_sdp()
    }
}

/// 音频回声播放器
pub struct AudioEchoPlayer {
    rtp_player: RtpPlayer,
}

impl AudioEchoPlayer {
    /// 创建新的音频回声播放器
    pub async fn new() -> Result<(Self, String), MediaPlayError> {
        let rtp_player = RtpPlayer::new(MediaKind::Audio).await?;
        let sdp = rtp_player.get_local_sdp()?;
        
        Ok((
            Self { rtp_player },
            sdp
        ))
    }
    
    /// 初始化播放器
    pub async fn initialize(&mut self) -> Result<(), MediaPlayError> {
        // 已经在new()中初始化了
        Ok(())
    }
    
    /// 设置远程SDP
    pub async fn set_remote_sdp(&mut self, remote_sdp: &str) -> Result<(), MediaPlayError> {
        self.rtp_player.set_remote_sdp(remote_sdp).await
    }
    
    /// 停止回声
    pub fn stop_echo(&mut self) {
        self.rtp_player.stop_echo()
    }
}

#[async_trait]
impl MediaPlayer for AudioEchoPlayer {
    fn media_kind(&self) -> MediaKind {
        MediaKind::Audio
    }
    
    fn payload_type(&self) -> u8 {
        0 // PCMU
    }
    
    fn clock_rate(&self) -> u32 {
        8000
    }
    
    async fn play_to_remote(&mut self, _peer_connection: Arc<PeerConnection>) -> Result<(), MediaPlayError> {
        Err(MediaPlayError::Sdp("AudioEchoPlayer不支持此操作".to_string()))
    }
    
    async fn start_echo(&mut self) -> Result<(), MediaPlayError> {
        self.rtp_player.start_audio_echo().await
    }
    
    async fn get_local_sdp(&self) -> Result<String, MediaPlayError> {
        self.rtp_player.get_local_sdp()
    }
}