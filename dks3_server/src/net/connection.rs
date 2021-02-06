use futures::SinkExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};

use dks3_proto::frame::{CipherMode, Frame, FrameDecoder, FrameEncoder};

pub struct Connection {
    close_tx: broadcast::Sender<()>,
    cipher_change_tx: mpsc::Sender<CipherMode>,
    inbound_frame_rx: mpsc::Receiver<Frame>,
    outbound_frame_tx: mpsc::Sender<Frame>,
}

impl Connection {
    pub fn start<Read, Write>(
        cipher_pair: (CipherMode, CipherMode),
        reader: Read,
        writer: Write,
    ) -> Connection
    where
        Read: AsyncRead + Unpin + Send + 'static,
        Write: AsyncWrite + Unpin + Send + 'static,
    {
        let (close_tx, mut close_rx) = broadcast::channel::<()>(1);
        let (cipher_change_tx, mut cipher_change_rx) = mpsc::channel::<CipherMode>(1);
        let (outbound_frame_tx, mut outbound_frame_rx) = mpsc::channel::<Frame>(10);
        let (inbound_frame_tx, inbound_frame_rx) = mpsc::channel::<Frame>(10);
        let (inbound_cipher, outbound_cipher) = cipher_pair;

        let mut frame_reader = FramedRead::new(reader, FrameDecoder::new(inbound_cipher, false));
        let mut frame_writer = FramedWrite::new(writer, FrameEncoder::new(outbound_cipher, true));

        let _io_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    cipher = cipher_change_rx.recv() => {
                        let new_cipher = cipher.unwrap();
                        frame_reader.decoder_mut().set_cipher_mode(new_cipher.clone());
                        frame_writer.encoder_mut().set_cipher_mode(new_cipher.clone());
                    }
                    inbound_frame = frame_reader.next() => {
                        match inbound_frame {
                            Some(Ok(frame)) => {
                                let _ = inbound_frame_tx.send(frame).await;
                            },
                            _ => break
                        }
                    }
                    outbound_frame = outbound_frame_rx.recv() => {
                        // TODO: handle result and close connection
                        let _ = frame_writer.send(outbound_frame.unwrap()).await;
                    }
                    _ = close_rx.recv() => {
                        break;
                    }
                }
            }
        });

        Connection {
            close_tx,
            cipher_change_tx,
            inbound_frame_rx,
            outbound_frame_tx,
        }
    }

    pub async fn change_cipher_mode(&mut self, cipher_mode: CipherMode) {
        let _ = self.cipher_change_tx.send(cipher_mode).await;
    }

    pub fn close(&self) {
        let _ = self.close_tx.send(());
    }

    pub async fn read_frame(&mut self) -> Option<Frame> {
        self.inbound_frame_rx.recv().await
    }

    pub async fn write_frame(&self, frame: Frame) {
        let _ = self.outbound_frame_tx.send(frame).await;
    }
}
