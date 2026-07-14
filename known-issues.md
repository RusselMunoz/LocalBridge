# LocalBridge — Known Issues (Status Update)

## [RESOLVED] 1. CPU-bound BGRA→YUV420 conversion
- **Resolved**: Replaced with SIMD-accelerated conversion using the `yuv` crate.

## [RESOLVED] 2. No dirty-region capture
- **Resolved**: Reconfigured capture settings to use `DirtyRegionSettings::ReportAndRender`.

## [RESOLVED] 3. Input injection is a stub
- **Resolved**: Active event injection implemented using `enigo`.

## [RESOLVED] 4. No encoder rate/quality control
- **Resolved**: Target bitrate set to 8 Mbps.

## [RESOLVED] 5. Capture rate limit & latency
- **Resolved**: Framerate increased to 60 FPS (16ms throttle). Integrated browser playback correction interval checking to eliminate accumulated WebRTC buffering lag.
