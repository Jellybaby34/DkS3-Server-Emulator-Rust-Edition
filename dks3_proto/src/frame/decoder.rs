use bytes::{Buf, BytesMut};
use thiserror::Error;
use tokio_util::codec::Decoder;
use tracing::info;

use crate::frame::crypto::CipherMode;
use crate::frame::{crypto, Frame};

#[derive(Debug, Error)]
pub enum FrameDecoderError {
    #[error("frame contained an invalid ciphertext")]
    InvalidCiphertext,

    #[error("frame header has mismatched packet size fields")]
    InvalidSize,

    #[error("i/o error while decoding frame")]
    Io {
        #[from]
        source: std::io::Error,
    },
}

#[derive(Debug)]
pub enum FrameDecoderState {
    Header,
    Data {
        length: usize,
        counter: u32,
        global_counter: u16,
    },
}

#[derive(Debug)]
pub struct FrameDecoder {
    cipher_mode: CipherMode,
    // If [LoginFrame]s decoded by this codec have 128 bits of zeroes trailing on the header.
    has_128b_trailer: bool,
    state: FrameDecoderState,
}

impl FrameDecoder {
    pub fn new(cipher_mode: CipherMode, has_128b_trailer: bool) -> Self {
        Self {
            cipher_mode,
            has_128b_trailer,
            state: FrameDecoderState::Header,
        }
    }

    fn decode_header(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<(usize, u16, u32)>, FrameDecoderError> {
        info!("Decoding header");
        let header_size = if self.has_128b_trailer {
            super::LOGIN_HEADER_SIZE + 16
        } else {
            super::LOGIN_HEADER_SIZE
        };

        if src.len() < header_size {
            src.reserve(header_size);
            return Ok(None);
        }

        let packet_length = src.get_u16();
        let global_counter = src.get_u16();
        let _unk1 = src.get_u16();

        let packet_length_u32_a = src.get_u32();
        let packet_length_u32_b = src.get_u32();

        if packet_length as u32 + 2 != packet_length_u32_a + 14
            || packet_length as u32 + 2 != packet_length_u32_b + 14
        {
            return Err(FrameDecoderError::InvalidSize);
        }

        let _unk2 = src.get_u32();
        let _unk3 = src.get_u32();
        let counter = src.get_u32_le();

        if self.has_128b_trailer {
            src.advance(16);
        }

        Ok(Some((
            packet_length as usize + 2 - header_size,
            global_counter,
            counter,
        )))
    }

    pub fn set_cipher_mode(&mut self, cipher_mode: CipherMode) {
        self.cipher_mode = cipher_mode;
    }
}

impl Decoder for FrameDecoder {
    type Item = Frame;
    type Error = FrameDecoderError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        info!("Decoding frame");
        let (length, global_counter, counter) = match self.state {
            FrameDecoderState::Header => match self.decode_header(src)? {
                Some((length, global_counter, counter)) => {
                    self.state = FrameDecoderState::Data {
                        length,
                        global_counter,
                        counter,
                    };
                    (length, global_counter, counter)
                }
                None => return Ok(None),
            },
            FrameDecoderState::Data {
                length,
                global_counter,
                counter,
            } => (length, global_counter, counter),
        };

        if src.len() < length as usize {
            return Ok(None);
        }

        self.state = FrameDecoderState::Header;

        let data = src.split_to(length);
        let decrypted_data = crypto::decrypt(&self.cipher_mode, &data)
            .map_err(|_| FrameDecoderError::InvalidCiphertext)?;

        Ok(Some(Frame {
            counter,
            global_counter,
            data: decrypted_data,
        }))
    }
}
