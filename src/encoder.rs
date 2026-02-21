use anyhow::Result;
use openh264::{
    encoder::{Encoder, EncoderConfig},
    formats::YUVBuffer,
    OpenH264API,
};

pub struct H264Encoder {
    inner:  Encoder,
    width:  usize,
    height: usize,
}

impl H264Encoder {
    pub fn new(width: usize, height: usize, fps: u32) -> Result<Self> {
        let api = OpenH264API::from_source();
        let config = EncoderConfig::new();
        Ok(Self {
            inner: Encoder::with_api_config(api, config)?,
            width,
            height,
        })
    }

    pub fn encode_bgra(&mut self, bgra: &[u8]) -> Result<Vec<u8>> {
        let yuv = bgra_to_yuv420(bgra, self.width, self.height);
        let bitstream = self.inner.encode(&yuv)?;

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

/// BGRA → planar YUV420 (BT.601).
fn bgra_to_yuv420(bgra: &[u8], w: usize, h: usize) -> YUVBuffer {
    let pixels = w * h;
    let mut data = vec![0u8; pixels + pixels / 2]; // Y + U + V planes

    let y_off = 0;
    let u_off = pixels;
    let v_off = pixels + pixels / 4;

    for row in 0..h {
        for col in 0..w {
            let i = (row * w + col) * 4;
            let b = bgra[i]     as f32;
            let g = bgra[i + 1] as f32;
            let r = bgra[i + 2] as f32;

            let y =  16.0 + 0.257*r + 0.504*g + 0.098*b;
            let u = 128.0 - 0.148*r - 0.291*g + 0.439*b;
            let v = 128.0 + 0.439*r - 0.368*g - 0.071*b;

            data[y_off + row * w + col] = y.clamp(0.0, 255.0) as u8;

            if row % 2 == 0 && col % 2 == 0 {
                let ci = (row / 2) * (w / 2) + (col / 2);
                data[u_off + ci] = u.clamp(0.0, 255.0) as u8;
                data[v_off + ci] = v.clamp(0.0, 255.0) as u8;
            }
        }
    }

    YUVBuffer::from_vec(data, w, h)
}