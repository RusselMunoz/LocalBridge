# How PixelBridge Works

PixelBridge is a high-performance remote desktop streaming tool that allows you to view and control your Windows screen from a web browser.

## High-Level Architecture

The system consists of a **Rust Server** (the "Host") and a **Web Browser** (the "Client"). They communicate using **WebRTC** for low-latency video and input.

```text
+---------------------+            WebRTC (Video)            +----------------------+
|    Windows Host     |  ----------------------------------> |  Web Browser Client  |
| (Screen + Encoder)  |  <---------------------------------- | (Video Player + JS)  |
+---------------------+           WebRTC (Input)             +----------------------+
          ^                                                            |
          |                                                            |
          +----------------------- HTTP Signaling ----------------------+
```

## The Data Flow

### 1. Screen Capture (src/capture.rs)
- Uses the **Windows Graphics Capture API** to grab raw pixel data directly from the desktop.
- This is extremely fast because it happens at the system level.
- Each frame is captured in **BGRA** format (Blue, Green, Red, Alpha).

### 2. Video Encoding (src/encoder.rs)
- Raw frames are huge, so we must compress them.
- We first convert **BGRA** to **YUV420** (a format that video encoders love).
- We use **OpenH264** to compress these frames into **H.264 NAL units**.
- This compression reduces the data size by over 90% while keeping it looking good.

### 3. Transport (src/main.rs)
- We use **WebRTC** (Web Real-Time Communication) to send the video.
- WebRTC is perfect for this because it's designed for sub-second latency (unlike standard video streaming like YouTube).
- Before video starts, there is a "handshake" (Signaling) where the browser and server exchange connection details.

### 4. Client Side (client/index.html)
- The browser receives the H.264 video stream.
- It uses the built-in browser video player to decode and show the screen.
- JavaScript listens for mouse clicks and key presses.
- These events are sent back to the server via a **WebRTC Data Channel**.

### 5. Input Injection (src/input.rs)
- The server receives the JSON messages from the browser (e.g., `{"type":"mouse_move","x":100,"y":200}`).
- It decodes these and (optionally) simulates real Windows mouse/keyboard events.

## Key Technologies Used
- **Rust:** For high-performance, safe concurrency.
- **Tokio:** The asynchronous engine that lets us capture video and run a web server at the same time.
- **Axum:** The web framework used for the signaling server.
- **WebRTC.rs:** A pure-Rust implementation of the WebRTC stack.
- **Windows-Capture:** A high-performance library for Windows screen grabbing.
- **OpenH264:** Cisco's industry-standard H.264 video encoder.
