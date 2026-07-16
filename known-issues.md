# LocalBridge — Known Issues & Resolution Log

## [RESOLVED] 1. CPU-bound BGRA→YUV420 conversion
- **Resolution**: Replaced nested scalar floating-point loop with `yuv` crate's SIMD-accelerated `bgra_to_yuv420` color space conversion using fixed-point integer math.

## [RESOLVED] 2. No dirty-region capture
- **Resolution**: Configured capture settings to use `DirtyRegionSettings::ReportAndRender` to render and transmit frames only when changes are detected.

## [RESOLVED] 3. Input injection is a stub / Enigo mouse move panic
- **Resolution**: Implemented Windows OS event simulation using `enigo = "0.1.3"`. Since Enigo's `mouse_move_to` absolute coordinates method panics on Windows when target windows are elevated or when hover events clash with UI Access boundaries, we bypassed Enigo's movement code and directly invoked the Windows User32 `SetCursorPos` API.

## [RESOLVED] 4. No encoder rate/quality control
- **Resolution**: Set explicit encoder config parameters, raising target streaming bitrate to 8 Mbps for high-quality LAN connections.

## [RESOLVED] 5. Capture rate limit & latency
- **Resolution**: Upgraded target capture framework to 60 FPS (16ms update pacing). Added browser-side WebRTC video element buffer clearing inside the client dashboard to dynamically skip queued frames if browser-side delay exceeds 180ms.

## [RESOLVED] 6. Real-time statistics reporting
- **Resolution**: Added floating status indicators showing real-time client decoding FPS and transport/buffer delay (in milliseconds). Integrated debug logs tracking frame encoding times and warnings when an encoding step exceeds the 16.6ms window for 60 FPS.
