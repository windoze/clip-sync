use std::future::Future;

use serde::{Deserialize, Serialize};

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageData {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

impl ImageData {
    pub fn from_png(bytes: &[u8]) -> anyhow::Result<Self> {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().expect("Failed to decode icon");
        let info = reader.info();
        let mut buf = vec![0; info.raw_bytes()];
        let output_info = reader
            .next_frame(buf.as_mut_slice())
            .expect("Failed to decode icon");
        output_info.buffer_size();

        Ok(Self {
            width: output_info.width as usize,
            height: output_info.height as usize,
            data: buf[0..output_info.buffer_size()].to_vec(),
        })
    }

    pub fn to_png(&self) -> anyhow::Result<Vec<u8>> {
        let mut buf = vec![];
        let mut encoder = png::Encoder::new(&mut buf, self.width as u32, self.height as u32);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&self.data)?;
        writer.finish()?;
        Ok(buf)
    }
}

impl std::fmt::Debug for ImageData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardContent {
    Text(String),
    Image(ImageData),
}

impl ClipboardContent {
    pub fn clear(&mut self) {
        *self = ClipboardContent::Text("".to_string());
    }

    pub fn is_empty(&self) -> bool {
        match self {
            ClipboardContent::Text(text) => text.is_empty(),
            ClipboardContent::Image(img) => img.data.is_empty(),
        }
    }
}

impl std::fmt::Debug for ClipboardContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClipboardContent::Text(text) => write!(f, "Text({})", text),
            ClipboardContent::Image(img) => write!(f, "Image({}x{})", img.width, img.height),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardRecord {
    pub source: String,
    pub content: ClipboardContent,
}

pub trait ClipboardSource {
    fn poll(&mut self) -> impl Future<Output = anyhow::Result<ClipboardRecord>>;
}

pub trait ClipboardSink {
    fn publish(
        &mut self,
        data: Option<ClipboardRecord>,
    ) -> impl Future<Output = anyhow::Result<()>>;
}

pub trait ClipSyncClient {
    type Config;
    fn connect(
        args: Self::Config,
    ) -> impl Future<Output = anyhow::Result<(String, impl ClipboardSource, impl ClipboardSink)>>;
}

#[cfg(feature = "websocket")]
mod ws {
    use serde::{Deserialize, Serialize};

    fn default_timestamp() -> i64 {
        chrono::Utc::now().timestamp()
    }

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct ClipboardMessage {
        #[serde(flatten)]
        pub entry: ServerClipboardRecord,
        #[serde(default = "default_timestamp")]
        pub timestamp: i64,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    pub struct ServerClipboardRecord {
        pub source: String,
        #[serde(flatten)]
        pub content: ServerClipboardContent,
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "lowercase")]
    pub enum ServerClipboardContent {
        Text(String),
        ImageUrl(String),
    }
}

#[cfg(feature = "websocket")]
pub use ws::*;
