use std::{sync::Arc, time::Duration};
use anyhow::Result;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use webrtc::{media::Sample, track::track_local::track_local_static_sample::TrackLocalStaticSample};
// 'windows_capture' is a library that provides high-performance screen capture on Windows.
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings,
        DirtyRegionSettings, MinimumUpdateIntervalSettings, SecondaryWindowSettings,
    },
};

use crate::encoder::H264Encoder;

// We want to capture and stream at 30 frames per second.
const TARGET_FPS: u32 = 30;
// One-based monitor index from windows-capture.
const CAPTURE_MONITOR_INDEX: usize = 1;

#[derive(Clone)]
struct CaptureFlags {
    track: Arc<TrackLocalStaticSample>,
    width: usize,
    height: usize,
}

fn select_capture_monitor() -> Result<Monitor> {
    match Monitor::from_index(CAPTURE_MONITOR_INDEX) {
        Ok(mon) => Ok(mon),
        Err(e) => {
            warn!(
                "Monitor #{CAPTURE_MONITOR_INDEX} unavailable ({e}); falling back to primary monitor"
            );
            Ok(Monitor::primary()?)
        }
    }
}

/// 'FrameHandler' is the core of our capture logic.
/// It implements 'GraphicsCaptureApiHandler', which means the 'windows-capture' 
/// library will call its methods whenever a new screen frame is ready.
struct FrameHandler {
    encoder: H264Encoder,
    track:   Arc<TrackLocalStaticSample>,
    rt:      tokio::runtime::Handle,
    frame_count: u64,
}

impl GraphicsCaptureApiHandler for FrameHandler {
    // These type aliases define what data we pass when creating a new handler.
    type Flags = CaptureFlags;
    type Error = anyhow::Error;

    /// 'new' is called when the capture starts.
    fn new(context: Context<Self::Flags>) -> Result<Self> {
        let flags = context.flags;
        
        // Initialize our H.264 encoder with the selected monitor's dimensions.
        Ok(Self {
            encoder: H264Encoder::new(flags.width, flags.height, TARGET_FPS)?,
            track: flags.track,
            // We store a handle to the Tokio runtime so we can spawn tasks from inside 
            // the capture callback (which runs on its own thread).
            rt: tokio::runtime::Handle::current(),
            frame_count: 0,
        })
    }

    /// 'on_frame_arrived' is called for every single frame captured from the screen.
    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _ctrl: InternalCaptureControl,
    ) -> Result<()> {
        // 1. Get the raw pixel data (BGRA format) from the frame.
        let mut buf = frame.buffer()?;
        let raw = buf.as_nopadding_buffer()?;
        
        // 2. Encode the raw pixels into an H.264 bitstream (NAL units).
        let nal = self.encoder.encode_bgra(raw)?;
        
        // If the encoder didn't produce any data yet (some encoders buffer a few frames), just wait.
        if nal.is_empty() { return Ok(()); }
        self.frame_count += 1;
        if self.frame_count == 1 || self.frame_count % 120 == 0 {
            info!("Encoded frame #{} ({} bytes)", self.frame_count, nal.len());
        }

        // 3. Send the encoded data to the WebRTC track.
        // We use 'rt.spawn' to move the network-sending work to an async task,
        // so we don't block the next frame from being captured.
        let track = self.track.clone();
        let dur   = Duration::from_secs(1) / TARGET_FPS;
        self.rt.spawn(async move {
            if let Err(e) = track.write_sample(&Sample {
                data:     nal.into(), // 'into()' converts Vec<u8> to Bytes
                duration: dur,
                ..Default::default()
            }).await {
                error!("write_sample: {e}");
            }
        });
        Ok(())
    }

    /// 'on_closed' is called when the capture session ends.
    fn on_closed(&mut self) -> Result<()> {
        debug!("Capture closed");
        Ok(())
    }
}

/// The 'run' function starts the whole capture process.
pub async fn run(
    track: Arc<TrackLocalStaticSample>,
    _tx:   broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    // Select the first monitor by index (with primary fallback).
    let mon = select_capture_monitor()?;
    let mon_index = mon.index().unwrap_or(CAPTURE_MONITOR_INDEX);
    let mon_name = mon.name().unwrap_or_else(|_| "Unknown".to_owned());
    let mon_device = mon.device_name().unwrap_or_else(|_| "Unknown".to_owned());
    let width = mon.width()? as usize;
    let height = mon.height()? as usize;
    info!("Capturing monitor #{mon_index}: {mon_name} ({mon_device}) {width}x{height}");
    
    // Configure the capture settings.
    let settings = Settings::new(
        mon,
        CursorCaptureSettings::WithCursor,    // Capture the mouse cursor too.
        DrawBorderSettings::WithoutBorder,    // Don't show the yellow capture border.
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,                   // We want BGRA format (Blue-Green-Red-Alpha).
        CaptureFlags { track, width, height }, // Pass track and size to keep encoder in sync.
    );

    // 'FrameHandler::start' is a blocking call that begins the capture loop.
    // We use 'spawn_blocking' because it uses a dedicated thread for heavy work.
    tokio::task::spawn_blocking(|| FrameHandler::start(settings))
        .await??;
        
    Ok(())
}
