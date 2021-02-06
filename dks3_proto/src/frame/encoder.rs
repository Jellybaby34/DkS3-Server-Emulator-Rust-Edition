use std::io::Error;

use bytes::{BufMut, BytesMut};
use thiserror::Error;
use tokio_util::codec::Encoder;

use crate::frame::Frame;
use std::convert::TryInto;

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

impl Encoder<Frame> for FrameEncoder {
    type Error = FrameEncoderError;

    fn encode(&mut self, item: Frame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let len: u16 = item.data.len().try_into().map_err(|_| FrameEncoderError::InvalidSize)?;

        dst.put_u16(len - 2);
        dst.put_u16(item.global_counter);
        dst.put_u16(0); // unk1

        dst.put_u32(len as u32 - 14);
        dst.put_u32(len as u32 - 14);
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