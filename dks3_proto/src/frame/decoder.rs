use std::error::Error;
use std::fmt::{Display, Formatter};

use bytes::{Buf, Bytes, BytesMut};
use thiserror::Error;
use tokio_util::codec::Decoder;
use crate::frame::Frame;
use crate::frame::encoder::FrameEncoderError;

#[derive(Debug, Error)]
pub enum FrameDecoderError {
    #[error("frame would produce invalid data when read")]
    InvalidData,

    #[error("frame header has mismatched packet size fields")]
    InvalidSize,

    #[error("i/o error while decoding frame")]
    Io {
        #[from]
        source: std::io::Error,
    },
}

pub const LOGIN_HEADER_SIZE: usize = 26;

pub enum FrameDecoderState {
    Header,
    Data {
        length: usize,
        counter: u32,
        global_counter: u16,
    },
}

pub struct FrameDecoder {
    // If [LoginFrame]s decoded by this codec have 128 bits of zeroes trailing on the header.
    has_128b_trailer: bool,
    state: FrameDecoderState,
}

impl FrameDecoder {
    pub fn new(has_128b_trailer: bool) -> Self {
        Self {
            has_128b_trailer,
            state: FrameDecoderState::Header
        }
    }

    fn decode_header(&mut self, src: &mut BytesMut) -> Result<Option<(usize, u16, u32)>, FrameDecoderError> {
        let header_size = if self.has_128b_trailer {
            LOGIN_HEADER_SIZE + 16
        } else {
            LOGIN_HEADER_SIZE
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

        if packet_length as u32 + 2 != packet_length_u32_a + 14 || packet_length as u32 + 2 != packet_length_u32_b + 14 {
            return Err(FrameDecoderError::InvalidSize);
        }

        let _unk2 = src.get_u32();
        let _unk3 = src.get_u32();
        let counter = src.get_u32_le();

        if self.has_128b_trailer {
            src.advance(16);
        }

        Ok(Some((packet_length as usize + 2, global_counter, counter)))
    }
}

impl Decoder for FrameDecoder {
    type Item = Frame;
    type Error = FrameDecoderError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let (length, global_counter, counter) = match self.state {
            FrameDecoderState::Header => match self.decode_header(src)? {
                Some((length, global_counter, counter)) => {
                    self.state = FrameDecoderState::Data { length, global_counter, counter };
                    (length, global_counter, counter)
                }
                None => return Ok(None),
            },
            FrameDecoderState::Data { length, global_counter, counter } => (length, global_counter, counter),
        };

        if src.len() < length as usize {
            return Ok(None);
        }

        self.state = FrameDecoderState::Header;

        Ok(Some(Frame {
            counter,
            global_counter,
            data: src.split_to(length),
        }))
    }
}