# LocalBridge — Architecture

Rust host + WebRTC + browser client, for low-latency screen sharing/remote control from a Windows host to a browser.

## Data Flow

```
+------------------------------+   WebRTC video (H.264, 60 FPS)  +-----------------------+
|          Rust Host           | ------------------------------> |   Browser Client      |
|  capture -> encode -> track  | <------------------------------ |  video player + JS    |
+------------------------------+       WebRTC data channel       +-----------------------+
                ^                       (mouse/key input)                    |
                |                                                            |
                +----------------- HTTP signaling (POST /offer) ------------+
```

## Components (updated)

### 1. `src/main.rs` — server + signaling entry point
- Serves HTTP signaling on `0.0.0.0:7878`.
- Exchanges SDP offers/answers to establish peer connections.
- Sets up WebRTC tracks and redirects remote input events via the data channel.

### 2. `src/capture.rs` — screen capture
- Captures at `TARGET_FPS = 60` with a pacing interval of 16ms (`MinimumUpdateIntervalSettings::Custom`).
- Uses `DirtyRegionSettings::ReportAndRender` to only capture when dirty region updates are available.

### 3. `src/encoder.rs` — H.264 encoding
- Converts raw BGRA8 to YUV420 using the SIMD-accelerated `yuv` crate (`yuv::bgra_to_yuv420` with `Balanced` conversion accuracy).
- Encodes YUV frames to H.264 using Cisco's OpenH264 library with target bitrates tuned for LAN streaming (8 Mbps).

### 4. `src/input.rs` — input handler
- Simulates real mouse movements, clicks, scrolls, and key presses on the Windows host using `enigo = "0.1.3"`.
- Scales client-side normalized coordinate inputs `(x, y)` to match the exact pixel dimensions of the host's primary monitor.

### 5. `client/index.html` — browser client dashboard
- Features a premium UI dashboard with a detailed connection checklist indicating signaling, WebRTC negotiation, and stream states.
- Applies client-side timing logic to monitor and prevent player buffer latency accumulation.
