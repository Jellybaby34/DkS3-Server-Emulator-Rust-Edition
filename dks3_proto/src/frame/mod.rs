use bytes::BytesMut;

pub use crypto::CipherMode;
pub use decoder::{FrameDecoder, FrameDecoderError};
pub use encoder::{FrameEncoder, FrameEncoderError};

mod crypto;
mod decoder;
mod encoder;

pub(crate) const LOGIN_HEADER_SIZE: usize = 26;

pub struct Frame {
    pub global_counter: u16,
    pub counter: u32,
    pub data: BytesMut,
}

impl Frame {
    pub fn new(global_counter: u16, counter: u32, data: BytesMut) -> Self {
        Self {
            global_counter,
            counter,
            data,
        }
    }
}
