use anyhow::Result;
use openh264::{
    encoder::{Encoder, EncoderConfig},
    formats::YUVBuffer,
    OpenH264API,
};

/// 'H264Encoder' handles converting raw images into compressed video.
pub struct H264Encoder {
    inner:  Encoder,
    width:  usize,
    height: usize,
    frame_index: u64,
}


impl H264Encoder {
    /// Creates a new encoder.
    pub fn new(width: usize, height: usize, _fps: u32) -> Result<Self> {
        // We use Cisco's OpenH264 library. 'from_source' will compile/link it for us.
        let api = OpenH264API::from_source();
        let config = EncoderConfig::new()
            .set_bitrate_bps(8_000_000)
            .max_frame_rate(_fps as f32);
        Ok(Self {
            inner: Encoder::with_api_config(api, config)?,
            width,
            height,
            frame_index: 0,
        })
    }

    /// Takes a raw BGRA buffer and returns a compressed H.264 bitstream.
    pub fn encode_bgra(&mut self, bgra: &[u8]) -> Result<Vec<u8>> {
        self.frame_index += 1;
        // Ensure late-joining peers quickly receive a decodable frame.
        // At 30 FPS, interval 60 ~= every 2 seconds.
        if self.frame_index == 1 || self.frame_index % 60 == 0 {
            self.inner.force_intra_frame();
        }

        // H.264 encoders usually don't accept BGRA (Red, Green, Blue, Alpha).
        // They require YUV420 format (Luminance and Chrominance).
        let yuv = bgra_to_yuv420(bgra, self.width, self.height);
        
        // The actual compression happens here.
        let bitstream = self.inner.encode(&yuv)?;

        // Extract the NAL units (Network Abstraction Layer) from the encoded bitstream.
        // These are the packets of video data that we send over the network.
        let mut out = Vec::new();
        let mut i = 0;
        while let Some(layer) = bitstream.layer(i) {
            let mut j = 0;
            while let Some(nal) = layer.nal_unit(j) {
                out.extend_from_slice(nal);
                j += 1;
            }
            i += 1;
        }
        Ok(out)
    }
}

/// Converts BGRA (8-bit Blue, Green, Red, Alpha) to planar YUV420.
/// Y = Brightness (Luma)
/// U/V = Color (Chroma)
/// 420 means we keep full resolution for Brightness, but half resolution for Color.
fn bgra_to_yuv420(bgra: &[u8], w: usize, h: usize) -> YUVBuffer {
    let pixels = w * h;
    let mut data = vec![0u8; pixels + pixels / 2]; 

    let (y_plane, rest) = data.split_at_mut(pixels);
    let (u_plane, v_plane) = rest.split_at_mut(pixels / 4);

    let mut planar = yuv::YuvPlanarImageMut {
        y_plane: yuv::BufferStoreMut::Borrowed(y_plane),
        y_stride: w as u32,
        u_plane: yuv::BufferStoreMut::Borrowed(u_plane),
        u_stride: (w / 2) as u32,
        v_plane: yuv::BufferStoreMut::Borrowed(v_plane),
        v_stride: (w / 2) as u32,
        width: w as u32,
        height: h as u32,
    };

    yuv::bgra_to_yuv420(
        &mut planar,
        bgra,
        (w * 4) as u32,
        yuv::YuvRange::Limited,
        yuv::YuvStandardMatrix::Bt601,
        yuv::YuvConversionMode::Balanced,
    ).expect("YUV conversion failed");

    YUVBuffer::from_vec(data, w, h)
}
