use std::{sync::Arc, time::Duration};
use anyhow::Result;
use tokio::sync::broadcast;
use tracing::{error, debug};
use webrtc::{media::Sample, track::track_local::track_local_static_sample::TrackLocalStaticSample};
use windows_capture::{
    capture::GraphicsCaptureApiHandler,
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    monitor::Monitor,
    settings::{
        ColorFormat, CursorCaptureSettings, DrawBorderSettings, Settings,
        DirtyRegionSettings, MinimumUpdateIntervalSettings, SecondaryWindowSettings,
    },
};

use crate::encoder::H264Encoder;

const TARGET_FPS: u32 = 30;

struct FrameHandler {
    encoder: H264Encoder,
    track:   Arc<TrackLocalStaticSample>,
    rt:      tokio::runtime::Handle,
}

impl GraphicsCaptureApiHandler for FrameHandler {
    type Flags = Arc<TrackLocalStaticSample>;
    type Error = anyhow::Error;

    fn new(track: Self::Flags) -> Result<Self> {
        let mon = Monitor::primary()?;
        let w   = mon.width()?  as usize;
        let h   = mon.height()? as usize;
        Ok(Self {
            encoder: H264Encoder::new(w, h, TARGET_FPS)?,
            track,
            rt: tokio::runtime::Handle::current(),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        _ctrl: InternalCaptureControl,
    ) -> Result<()> {
        let buf = frame.buffer()?;
        let raw = buf.as_raw_nopadding_buffer()?;
        let nal = self.encoder.encode_bgra(raw)?;
        if nal.is_empty() { return Ok(()); }

        let track = self.track.clone();
        let dur   = Duration::from_secs(1) / TARGET_FPS;
        self.rt.spawn(async move {
            if let Err(e) = track.write_sample(&Sample {
                data:     nal.into(),
                duration: dur,
                ..Default::default()
            }).await {
                error!("write_sample: {e}");
            }
        });
        Ok(())
    }

    fn on_closed(&mut self) -> Result<()> {
        debug!("Capture closed");
        Ok(())
    }
}

pub async fn run(
    track: Arc<TrackLocalStaticSample>,
    _tx:   broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    let mon = Monitor::primary()?;
    let settings = Settings::new(
        mon,
        CursorCaptureSettings::WithCursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        track,
    );
    // Runs blocking — windows-capture calls our handler on each frame.
    tokio::task::spawn_blocking(|| FrameHandler::start(settings))
        .await??;
    Ok(())
}