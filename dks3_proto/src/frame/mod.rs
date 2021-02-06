use bytes::BytesMut;

mod decoder;
mod encoder;

pub use decoder::FrameDecoder;

pub struct Frame {
    pub global_counter: u16,
    pub counter: u32,
    pub data: BytesMut,
}

