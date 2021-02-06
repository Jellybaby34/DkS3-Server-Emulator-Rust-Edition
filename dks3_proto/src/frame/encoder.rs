use std::convert::TryInto;

use bytes::{BufMut, BytesMut};
use thiserror::Error;
use tokio_util::codec::Encoder;

use crate::frame::crypto::{self, CipherMode};
use crate::frame::Frame;

pub struct FrameEncoder {
    cipher_mode: CipherMode,
    has_128b_trailer: bool,
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
    pub fn new(cipher_mode: CipherMode, has_128b_trailer: bool) -> Self {
        FrameEncoder {
            cipher_mode,
            has_128b_trailer,
        }
    }

    pub fn set_cipher_mode(&mut self, cipher_mode: CipherMode) {
        self.cipher_mode = cipher_mode;
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

        let encrypted_data = crypto::encrypt(&self.cipher_mode, &item.data).unwrap();
        let total_len = (encrypted_data.len() + header_size) as u16;

        dst.put_u16(total_len - 2);
        dst.put_u16(item.global_counter);
        dst.put_u16(0); // unk1

        dst.put_u32(total_len as u32 - 14);
        dst.put_u32(total_len as u32 - 14);
        dst.put_u32(0x0c); // unk2
        dst.put_u32(0); // unk3
        dst.put_u32_le(item.counter);

        if self.has_128b_trailer {
            dst.put_u32(0);
            dst.put_u32(1);
            dst.put_u32(0);
            dst.put_u32(0);
        }

        dst.put(&encrypted_data[..]);

        Ok(())
    }
}
