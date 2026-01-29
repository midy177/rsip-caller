use clap::Parser;
use sip_caller::{create_sip_client_with_proxy, create_audio_player, create_video_player, create_rtp_session, MediaKind, utils};
use sip_caller::rtp_play::{AudioEchoPlayer, MediaPlayer};
use std::io::{self, Write};

use std::time::Duration;
use rsipstack::dialog::dialog::{DialogState, TerminatedReason};

use tracing::{info, error};


/// SIP Caller CLI Application
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// SIP server address (e.g., 127.0.0.1:5060)
    #[arg(short, long)]
    server: Option<String>,
    
    /// SIP username (e.g., user@example.com)
    #[arg(short, long)]
    user: Option<String>,
    
    /// SIP password
    #[arg(short, long)]
    password: Option<String>,
    
    /// Outbound proxy server (e.g., sip:proxy.example.com:5060;transport=udp;lr)
    #[arg(short, long)]
    outbound_proxy: Option<String>,
    
    /// Media file path for RTP streaming
    #[arg(short = 'f', long)]
    media: Option<String>,
    
    /// Media type (audio/video)
    #[arg(long, default_value = "auto")]
    media_type: String,
    
    /// Operation mode (call/echo/media)
    #[arg(short, long, default_value = "call")]
    mode: String,
    
    /// Call target (user@domain)
    #[arg(short, long)]
    target: Option<String>,
    
    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    utils::initialize_logging(args.log_level.as_str());
    match args.mode.as_str() {
        "call" => run_call_mode(&args).await,
        "echo" => run_echo_mode(&args).await,
        "media" => run_media_mode(&args).await,
        _ => {
            eprintln!("Invalid mode. Use 'call', 'echo', or 'media'");
            Ok(())
        }
    }
}

async fn run_call_mode(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let server = args.server.clone()
        .or_else(|| std::env::var("SIP_SERVER").ok())
        .ok_or("SIP server address is required")?;
    
    let user = args.user.clone()
        .or_else(|| std::env::var("SIP_USER").ok())
        .ok_or("SIP user is required")?;
    
    let password = args.password.clone()
        .or_else(|| std::env::var("SIP_PASSWORD").ok())
        .unwrap_or_else(|| "password".to_string());
    
    let target = args.target.clone()
        .or_else(|| std::env::var("SIP_TARGET").ok())
        .ok_or("Call target is required in call mode")?;

    info!("Creating SIP client for {}: {}", server, user);
    
    let client = create_sip_client_with_proxy(
        &server, 
        &user, 
        &password, 
        args.outbound_proxy.as_deref()
    ).await?;
    
    if let Some(media_path) = &args.media {
        let media_type = detect_media_type(media_path, &args.media_type)?;
        
        info!("Creating RTP session for media: {}", media_path);
        let (mut rtp_player, local_sdp) = create_rtp_session(media_type).await?;
        
        println!("Local SDP:");
        println!("{}", local_sdp);
        
        print!("Enter remote SDP: ");
        io::stdout().flush()?;
        let mut remote_sdp = String::new();
        io::stdin().read_line(&mut remote_sdp)?;
        
        let media_player = match media_type {
            MediaKind::Audio => create_audio_player(media_path).await?,
            MediaKind::Video => create_video_player(media_path).await?,
        };
        
        rtp_player.set_remote_sdp_and_play(&remote_sdp, media_player).await?;
        
        info!("Media streaming completed");
    } else {
        match client.register().await {
            Ok(response) => {
                info!("SIP registration completed successfully");
                info!("Registration response: {}", response.status_code);
            }
            Err(e) => {
                error!("SIP registration failed: {}", e);
                error!("Error code: {}", e.error_code());
                return Err(format!("SIP registration failed: {}", e).into());
            }
        }
        
        match client.make_call(&target, "").await {
            Ok((dialog, response)) => {
                info!("Call initiated successfully");
                info!("Dialog ID: {:?}", dialog.id());
                if let Some(resp) = response {
                    info!("Call response: {}", resp.status_code);
                }
            }
            Err(e) => {
                error!("Call failed: {}", e);
                error!("Error code: {}", e.error_code());
                return Err(format!("Call failed: {}", e).into());
            }
        }
        
        tokio::signal::ctrl_c().await?;
        info!("Shutting down...");
    }
    
    Ok(())
}

async fn run_echo_mode(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let server = args.server.clone()
        .or_else(|| std::env::var("SIP_SERVER").ok())
        .ok_or("SIP server address is required")?;
    
    let user = args.user.clone()
        .or_else(|| std::env::var("SIP_USER").ok())
        .ok_or("SIP user is required")?;
    
    let password = args.password.clone()
        .or_else(|| std::env::var("SIP_PASSWORD").ok())
        .unwrap_or_else(|| "password".to_string());

    let target = args.target.clone()
        .or_else(|| std::env::var("SIP_TARGET").ok())
        .ok_or("Call target is required in call mode")?;

    info!("Creating SIP client for echo mode: {}@{}", user, server);

    let client = create_sip_client_with_proxy(
        &server, 
        &user, 
        &password, 
        args.outbound_proxy.as_deref()
    ).await?;
    
    // Register with SIP server
    match client.register().await {
            Ok(response) => {
                info!("SIP registration completed successfully");
                info!("Registration response: {}", response.status_code);
            }
            Err(e) => {
                error!("SIP registration failed: {}", e);
                error!("Error code: {}", e.error_code());
                return Err(format!("SIP registration failed: {}", e).into());
            }
        }
    
    // Handle echo calls
    run_echo_mode_real(&client,&target).await?;
    Ok(())
}

async fn run_echo_mode_real(client: &sip_caller::SipClient, target: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Echo mode: making call to: {}", target);

    // Create echo player
    let (mut echo_player, local_sdp) = AudioEchoPlayer::new().await?;

    // Make call to target with SDP offer
    info!("Making echo call to: {}", target);
    match client.make_call(&target, &local_sdp).await {
        Ok((dialog, response)) => {
            info!("Call initiated successfully");
            info!("Dialog ID: {:?}", dialog.id());
            
            // Process response to get remote SDP
            let remote_sdp = if let Some(resp) = response {
                info!("Received response: {}", resp.status_code);
                
                // Extract SDP answer from response
                let sdp_answer = extract_sdp_from_response(&resp)?;
                if !sdp_answer.is_empty() {
                    info!("Received SDP answer in response: {}", sdp_answer);
                    Some(sdp_answer)
                } else {
                    // We'll need to wait for a re-INVITE with SDP or for SDP in ACK
                    info!("No SDP in 200 OK, waiting for subsequent messages");
                    None
                }
            } else {
                info!("No response received");
                None
            };
            
            // If we got SDP, parse it to get the remote RTP address, otherwise wait for it
            let final_remote_sdp = if let Some(sdp) = remote_sdp {
                sdp
            } else {
                info!("Enter remote SDP for echo: ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim().is_empty() {
                    error!("No SDP provided");
                    return Ok(());
                }
                input
            };

            // Initialize echo player
            if let Err(e) = echo_player.initialize().await {
                error!("Failed to initialize echo player: {}", e);
                return Err(format!("Failed to initialize echo player: {}", e).into());
            }
            
            // Set remote SDP
            if let Err(e) = echo_player.set_remote_sdp(&final_remote_sdp).await {
                error!("Failed to set remote SDP: {}", e);
                return Err(format!("Failed to set remote SDP: {}", e).into());
            }
            
            // Start echo mode
            info!("Starting echo mode with SDP");

            // Start echo mode using AudioEchoPlayer
            // AudioEchoPlayer uses its internal PeerConnection
            if let Err(e) = echo_player.start_echo().await {
                error!("Echo mode failed: {}", e);
                return Err(format!("Echo mode failed: {}", e).into());
            }
            info!("Echo mode active: audio will be echoed back to the caller");
            // Wait for cancellation
            loop {
                match dialog.state() {
                    DialogState::Terminated(_, TerminatedReason::UasBye) => {
                        info!("对端主动挂断");
                        echo_player.stop_echo();
                        break;
                    }
                    DialogState::Terminated(_, reason) => {
                        info!("通话结束: {:?}", reason);
                        echo_player.stop_echo();
                        break;
                    }
                    _ => {
                        // 继续等待
                    }
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
            info!("Echo mode completed");
            Ok(())
        }
        Err(e) => {
            error!("Failed to make echo call: {}", e);
            error!("Error code: {}", e.error_code());
            return Err(format!("Echo call failed: {}", e).into());
        }
    }
}

// Helper function to extract SDP from response
fn extract_sdp_from_response(response: &rsip::Response) -> Result<String, Box<dyn std::error::Error>> {
    match response.status_code {
        rsip::StatusCode::OK => {
            let body = response.body();
            if !body.is_empty() {
                Ok(String::from_utf8_lossy(&body).to_string())
            } else {
                Err("No SDP in OK response".into())
            }
        }
        rsip::StatusCode::Ringing => {
            Err("Call is still ringing, no SDP yet".into())
        }
        _ => {
            Err(format!("Call failed with status: {}", response.status_code).into())
        }
    }
}

async fn run_media_mode(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let server = args.server.clone()
        .or_else(|| std::env::var("SIP_SERVER").ok())
        .ok_or("SIP server address is required")?;
    
    let user = args.user.clone()
        .or_else(|| std::env::var("SIP_USER").ok())
        .ok_or("SIP user is required")?;
    
    let password = args.password.clone()
        .or_else(|| std::env::var("SIP_PASSWORD").ok())
        .unwrap_or_else(|| "password".to_string());
    
    let target = args.target.clone()
        .or_else(|| std::env::var("SIP_TARGET").ok())
        .ok_or("Call target is required in media mode")?;
    
    let media_file = args.media.clone()
        .ok_or("Media file path is required in media mode")?;
    
    info!("Creating SIP client for media mode: {}@{}", user, server);
    
    let client = create_sip_client_with_proxy(
        &server, 
        &user, 
        &password, 
        args.outbound_proxy.as_deref()
    ).await?;
    
    // Register with SIP server
    match client.register().await {
            Ok(response) => {
                info!("SIP registration completed successfully");
                info!("Registration response: {}", response.status_code);
            }
            Err(e) => {
                error!("SIP registration failed: {}", e);
                error!("Error code: {}", e.error_code());
                return Err(format!("SIP registration failed: {}", e).into());
            }
        }
    
    // Create RTP session for media
    let media_type_str = args.media_type.as_str();
    let media_type = detect_media_type(&media_file, media_type_str)?;
    let (mut rtp_player, local_sdp) = create_rtp_session(media_type).await?;
    
    println!("Local SDP for media mode:");
    println!("{}", local_sdp);
    
    // Make call to target with SDP offer
    info!("Making media call to: {}", target);
    match client.make_call(&target, &local_sdp).await {
        Ok((dialog, response)) => {
            info!("Call initiated successfully");
            info!("Dialog ID: {:?}", dialog.id());
            
            // Process response to get remote SDP
            let remote_sdp = if let Some(resp) = response {
                info!("Received response: {}", resp.status_code);
                
                // Extract SDP answer from response
                let sdp_answer = extract_sdp_from_response(&resp)?;
                if !sdp_answer.is_empty() {
                    info!("Received SDP answer in response");
                    Some(sdp_answer)
                } else {
                    // We'll need to wait for a re-INVITE with SDP or for SDP in ACK
                    info!("No SDP in 200 OK, waiting for subsequent messages");
                    None
                }
            } else {
                info!("No response received");
                None
            };
            
            // If we got SDP, parse it to get the remote RTP address, otherwise wait for it
            let final_remote_sdp = if let Some(sdp) = remote_sdp {
                sdp
            } else {
                print!("Enter remote SDP for media: ");
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                if input.trim().is_empty() {
                    error!("No SDP provided");
                    return Ok(());
                }
                input
            };
            
            // Create media player
            let media_player = if media_type == MediaKind::Audio {
                create_audio_player(&media_file).await?
            } else {
                create_video_player(&media_file).await?
            };
            
            // Start media playback
            info!("Starting media playback with SDP");
            rtp_player.set_remote_sdp_and_play(&final_remote_sdp, media_player).await?;
            
            info!("Media playback active");
            info!("Press Ctrl+C to exit");
            
            // Keep client running
            tokio::signal::ctrl_c().await?;
            info!("Shutting down...");
            
            Ok(())
        }
        Err(e) => {
            error!("Failed to make media call: {}", e);
            error!("Error code: {}", e.error_code());
            return Err(format!("Media call failed: {}", e).into());
        }
    }
}
// Helper function to detect media type
fn detect_media_type(file_path: &str, media_type: &str) -> Result<MediaKind, Box<dyn std::error::Error>> {
    if media_type == "auto" {
        let path = std::path::Path::new(file_path);
        let ext = path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        match ext.as_str() {
            "wav" => Ok(MediaKind::Audio),
            "ivf" => Ok(MediaKind::Video),
            _ => Err(Box::from("Unsupported media format")),
        }
    } else {
        match media_type {
            "audio" => Ok(MediaKind::Audio),
            "video" => Ok(MediaKind::Video),
            _ => Err(Box::from("Invalid media type")),
        }
    }
}