// These 'mod' declarations tell Rust to look for other files in this project.
// For example, 'mod capture' looks for capture.rs and makes its contents available here.
mod capture;
mod encoder;
mod input;
mod signaling;

// 'use' statements are like imports in other languages. 
// They bring external or internal items into the current scope.
use std::{collections::HashMap, net::IpAddr, sync::Arc};
use anyhow::Result;
use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use tokio::sync::{broadcast, Mutex};
use tower_http::cors::CorsLayer;
use tracing::info;
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::{MediaEngine, MIME_TYPE_H264},
        setting_engine::SettingEngine,
        APIBuilder,
    },
    ice::network_type::NetworkType,
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration,
        sdp::session_description::RTCSessionDescription,
        RTCPeerConnection,
    },
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::{
        track_local_static_sample::TrackLocalStaticSample, TrackLocal,
    },
};

/// 'AppState' holds the shared data that our web server needs access to.
/// We use 'Arc' (Atomic Reference Counted) to allow multiple parts of the program 
/// to own and share this data safely across threads.
#[derive(Clone)]
pub struct AppState {
    // The WebRTC video track that we will push screen frames into.
    pub video_track: Arc<TrackLocalStaticSample>,
    // A list of connected peers. 'Mutex' ensures only one thread can modify this list at a time.
    pub peers:       Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    // A broadcast channel to send frames to multiple listeners if needed.
    pub frame_tx:    broadcast::Sender<Vec<u8>>,
}

/// The 'main' function is the entry point of the program.
/// '#[tokio::main]' sets up an asynchronous runtime, which allows us to run many 
/// tasks (like capture and web serving) concurrently.
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging so we can see what's happening in the console.
    tracing_subscriber::fmt()
        .with_env_filter("info,pixelbridge=debug,localbridge=debug,webrtc=error")
        .init();

    // Create the video track. This is the "pipe" through which our video data flows.
    let video_track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_H264.to_owned(),
            clock_rate: 90000,
            // Baseline profile with packetization-mode=1 has broad browser support.
            sdp_fmtp_line: "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f".to_owned(),
            ..Default::default()
        },
        "video".to_owned(),
        "localbridge".to_owned(),
    ));

    // Create a broadcast channel for internal frame distribution.
    let (frame_tx, _) = broadcast::channel::<Vec<u8>>(32);

    // Initialize our shared state.
    let state = AppState {
        video_track: video_track.clone(),
        peers:       Arc::new(Mutex::new(HashMap::new())),
        frame_tx:    frame_tx.clone(),
    };

    // Spawn the screen capture loop on its own asynchronous task.
    // 'tokio::spawn' runs this in the background while the rest of 'main' continues.
    let track_for_capture = video_track.clone();
    let tx_clone = frame_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = capture::run(track_for_capture, tx_clone).await {
            tracing::error!("Capture loop error: {e}");
        }
    });

    // Define our web server routes.
    // - "/" serves the HTML/JS client.
    // - "/offer" handles the WebRTC handshake.
    // - "/ws/input" is a WebSocket for control messages.
    let app = Router::new()
        .route("/",         get(serve_client))
        .route("/offer",    post(handle_offer))
        .route("/ws/input", get(signaling::ws_input_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    // Bind the server to all network interfaces on port 7878.
    let addr = "0.0.0.0:7878";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("LocalBridge listening on http://{addr}");
    info!("Open browser at http://<YOUR-LAN-IP>:7878");

    // Start serving requests.
    axum::serve(listener, app).await?;
    Ok(())
}

/// Serves the embedded 'index.html' file to the browser.
async fn serve_client() -> impl IntoResponse {
    axum::response::Html(include_str!("../client/index.html"))
}

/// Defines the structure of the JSON we expect when a client sends a WebRTC offer.
#[derive(serde::Deserialize)]
struct OfferBody {
    sdp:  String,
    #[serde(rename = "type")]
    _kind: String,
}

/// Axum handler for the POST /offer route.
async fn handle_offer(
    State(state): State<AppState>,
    Json(body):   Json<OfferBody>,
) -> impl IntoResponse {
    // We delegate the actual logic to 'do_offer'.
    match do_offer(state, body).await {
        Ok(ans) => Json(serde_json::json!({ "sdp": ans.sdp, "type": "answer" })),
        Err(e)  => {
            tracing::error!("Offer error: {e}");
            Json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

/// Performs the WebRTC handshake: receives an offer, sets up a connection, and returns an answer.
async fn do_offer(state: AppState, body: OfferBody) -> Result<RTCSessionDescription> {
    let remote_candidate_count = body.sdp.matches("a=candidate:").count();
    info!("Received offer with {remote_candidate_count} ICE candidate(s)");
    if remote_candidate_count == 0 {
        anyhow::bail!("Offer contained 0 ICE candidates. Refresh the client and retry.");
    }
    info!(
        "Offer codec hints: h264={}, vp8={}",
        body.sdp.contains("H264/90000"),
        body.sdp.contains("VP8/90000")
    );

    // Re-configure the MediaEngine for this specific connection.
    let mut me = MediaEngine::default();
    me.register_default_codecs()?;
    let reg = Registry::new();
    let reg = register_default_interceptors(reg, &mut me)?;
    let mut se = SettingEngine::default();
    // WLAN/LAN default: avoid IPv6/STUN-related gather errors on Windows and keep host UDP/IPv4 candidates.
    se.set_network_types(vec![NetworkType::Udp4]);
    se.set_ip_filter(Box::new(|ip: IpAddr| match ip {
        IpAddr::V4(v4) => !v4.is_link_local() && !v4.is_unspecified(),
        IpAddr::V6(v6) => !v6.is_loopback() && !v6.is_multicast() && !v6.is_unspecified(),
    }));
    let api = APIBuilder::new()
        .with_setting_engine(se)
        .with_media_engine(me)
        .with_interceptor_registry(reg)
        .build();

    // LAN/WLAN mode: host candidates only (no public STUN/TURN).
    let config = RTCConfiguration {
        ice_servers: vec![],
        ..Default::default()
    };

    // Create a new PeerConnection.
    let pc = Arc::new(api.new_peer_connection(config).await?);
    
    // Add our shared video track to this new connection so the client can see the screen.
    pc.add_track(Arc::clone(&state.video_track) as Arc<dyn TrackLocal + Send + Sync>).await?;

    // Set up a Data Channel to receive mouse/keyboard input from the client.
    pc.on_data_channel(Box::new(|dc| {
        Box::pin(async move { input::handle_data_channel(dc).await; })
    }));

    // Store the connection in our state.
    let id = uuid::Uuid::new_v4().to_string();
    state.peers.lock().await.insert(id.clone(), pc.clone());

    // Process the SDP offer from the client.
    let offer = RTCSessionDescription::offer(body.sdp)?;
    pc.set_remote_description(offer).await?;
    
    // Create an answer to send back to the client.
    let answer = pc.create_answer(None).await?;
    
    // Wait for the ICE gathering to complete so we have all necessary network info.
    let mut gather = pc.gathering_complete_promise().await;
    pc.set_local_description(answer).await?;
    let _ = gather.recv().await;

    // Return the final local description (the "answer").
    let local = pc.local_description().await
        .ok_or_else(|| anyhow::anyhow!("No local description"))?;
    let local_candidate_count = local.sdp.matches("a=candidate:").count();
    if local_candidate_count == 0 {
        anyhow::bail!("Server gathered 0 ICE candidates. Verify WLAN interface is up.");
    }
    info!(
        "Answer codec hints: h264={}, vp8={}",
        local.sdp.contains("H264/90000"),
        local.sdp.contains("VP8/90000")
    );
    info!("Peer {id} connected (remote={remote_candidate_count}, local={local_candidate_count} candidates)");
    Ok(local)
}
