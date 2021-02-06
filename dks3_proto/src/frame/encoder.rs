use std::convert::TryInto;

use bytes::{BufMut, BytesMut};
use thiserror::Error;
use tokio_util::codec::Encoder;

use crate::frame::Frame;

pub struct FrameEncoder {
    has_128b_trailer: bool
}

#[derive(Debug, Error)]
pub enum FrameEncoderError {
    #[error("frame data exceeded max size")]
    InvalidSize,

    #[error("i/o error while encoding frame")]
    Io {
        #[from]
        source: std::io::Error,
    },
}

impl FrameEncoder {
    pub fn new(has_128b_trailer: bool) -> Self {
        FrameEncoder {
            has_128b_trailer
        }
    }
}

impl Encoder<Frame> for FrameEncoder {
    type Error = FrameEncoderError;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let header_size = if self.has_128b_trailer {
            super::LOGIN_HEADER_SIZE + 16
        } else {
            super::LOGIN_HEADER_SIZE
        };

        let data_len: u16 = item.data.len().try_into().map_err(|_| FrameEncoderError::InvalidSize)?;
        let total_len = data_len + header_size as u16;

        dst.put_u16(total_len - 2);
        dst.put_u16(item.global_counter);
        dst.put_u16(0); // unk1

        dst.put_u32(total_len as u32 - 14);
        dst.put_u32(total_len as u32 - 14);
        dst.put_u32(0); // unk2
        dst.put_u32(0); // unk3
        dst.put_u32_le(item.counter);

        if self.has_128b_trailer {
            dst.put_u128(0);
        }

        dst.put(item.data);

        Ok(())
    }
}