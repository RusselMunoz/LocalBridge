mod capture;
mod encoder;
mod input;
mod signaling;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};
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
        APIBuilder,
    },
    ice_transport::ice_server::RTCIceServer,
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

#[derive(Clone)]
pub struct AppState {
    pub video_track: Arc<TrackLocalStaticSample>,
    pub peers:       Arc<Mutex<HashMap<String, Arc<RTCPeerConnection>>>>,
    pub frame_tx:    broadcast::Sender<Vec<u8>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("pixelbridge=debug,webrtc=warn")
        .init();

    let mut me = MediaEngine::default();
    me.register_default_codecs()?;
    let mut reg = Registry::new();
    reg = register_default_interceptors(reg, &mut me)?;

    let video_track = Arc::new(TrackLocalStaticSample::new(
        RTCRtpCodecCapability {
            mime_type: MIME_TYPE_H264.to_owned(),
            ..Default::default()
        },
        "video".to_owned(),
        "pixelbridge".to_owned(),
    ));

    let (frame_tx, _) = broadcast::channel::<Vec<u8>>(32);

    let state = AppState {
        video_track: video_track.clone(),
        peers:       Arc::new(Mutex::new(HashMap::new())),
        frame_tx:    frame_tx.clone(),
    };

    // Capture loop on its own thread.
    let track_for_capture = video_track.clone();
    let tx_clone = frame_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = capture::run(track_for_capture, tx_clone).await {
            tracing::error!("Capture loop error: {e}");
        }
    });

    let app = Router::new()
        .route("/",         get(serve_client))
        .route("/offer",    post(handle_offer))
        .route("/ws/input", get(signaling::ws_input_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = "0.0.0.0:7878";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("PixelBridge listening on http://{addr}");
    info!("Open browser at http://<YOUR-LAN-IP>:7878");

    axum::serve(listener, app).await?;
    Ok(())
}

async fn serve_client() -> impl IntoResponse {
    axum::response::Html(include_str!("../client/index.html"))
}

#[derive(serde::Deserialize)]
struct OfferBody {
    sdp:  String,
    #[serde(rename = "type")]
    kind: String,
}

async fn handle_offer(
    State(state): State<AppState>,
    Json(body):   Json<OfferBody>,
) -> impl IntoResponse {
    match do_offer(state, body).await {
        Ok(ans) => Json(serde_json::json!({ "sdp": ans.sdp, "type": "answer" })),
        Err(e)  => {
            tracing::error!("Offer error: {e}");
            Json(serde_json::json!({ "error": e.to_string() }))
        }
    }
}

async fn do_offer(state: AppState, body: OfferBody) -> Result<RTCSessionDescription> {
    let mut me = MediaEngine::default();
    me.register_default_codecs()?;
    let mut reg = Registry::new();
    reg = register_default_interceptors(reg, &mut me)?;
    let api = APIBuilder::new()
        .with_media_engine(me)
        .with_interceptor_registry(reg)
        .build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let pc = Arc::new(api.new_peer_connection(config).await?);
    pc.add_track(Arc::clone(&state.video_track) as Arc<dyn TrackLocal + Send + Sync>).await?;

    pc.on_data_channel(Box::new(|dc| {
        Box::pin(async move { input::handle_data_channel(dc).await; })
    }));

    let id = uuid::Uuid::new_v4().to_string();
    state.peers.lock().await.insert(id.clone(), pc.clone());

    let offer = RTCSessionDescription::offer(body.sdp)?;
    pc.set_remote_description(offer).await?;
    let answer = pc.create_answer(None).await?;
    let mut gather = pc.gathering_complete_promise().await;
    pc.set_local_description(answer).await?;
    let _ = gather.recv().await;

    let local = pc.local_description().await
        .ok_or_else(|| anyhow::anyhow!("No local description"))?;
    info!("Peer {id} connected");
    Ok(local)
}