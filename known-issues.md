# LocalBridge — Current Known Issues & Bottlenecks

While the initial stubs and crashes have been resolved, several critical performance bottlenecks remain, causing the current latency and delay.

---

## 1. Single-Threaded CPU YUV Conversion Bottleneck
- **Status:** **Active / High Priority**
- **File:** `src/capture.rs` and `src/encoder.rs`
- **Problem:** Although we upgraded to the SIMD-accelerated `yuv` crate, the color conversion (`bgra_to_yuv420`) is still executed synchronously on a single CPU core within the frame capture thread. For a 1920x1080 frame, this single-threaded math takes anywhere from `15ms` to `30ms` depending on CPU clock speeds.
- **Impact:** Directly caps the maximum frame rate well below 60 FPS and introduces cumulative capture thread latency.

## 2. Software H.264 Encoding Overhead
- **Status:** **Active / High Priority**
- **File:** `src/encoder.rs`
- **Problem:** Cisco's OpenH264 is a software-based encoder. Running H.264 compression on the CPU for high-resolution video streams consumes substantial CPU cycles.
- **Impact:** Compressing 1080p frames on the CPU frequently spikes frame encoding times up to `50ms–100ms+` during periods of high motion (such as scrolling or playing videos), causing severe streaming delays.

## 3. Lack of GPU Hardware Acceleration (NVENC / AMF / QuickSync)
- **Status:** **Active / Medium Priority**
- **Problem:** There is no GPU-accelerated encoding pipeline. Modern remote desktop tools (like Sunshine or Parsec) offload video encoding to dedicated hardware encoders on the graphics card (e.g., NVIDIA NVENC, AMD AMF, or Intel QuickSync).
- **Impact:** Without hardware encoding, achieving sub-10ms capture-to-display latency is practically impossible at 1080p or higher resolutions.

## 4. UIPI Restrictions on Administrative Windows
- **Status:** **Active / Low Priority**
- **File:** `src/input.rs`
- **Problem:** Although we transitioned to `SetCursorPos` to resolve mouse movement panics, Windows User Interface Privilege Isolation (UIPI) still blocks lower-privilege processes from sending keyboard/click inputs to elevated or administrative windows (e.g., Task Manager).
- **Impact:** Keystrokes and clicks will not register when an Administrator window has focus unless the `LocalBridge` host executable is run with Administrator privileges.
