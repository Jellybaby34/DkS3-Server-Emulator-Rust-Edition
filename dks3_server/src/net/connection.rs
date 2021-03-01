use futures::{SinkExt, TryStreamExt};
use tokio::io::{split, AsyncRead, AsyncWrite};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::info;

use dks3_proto::frame::{CipherMode, Frame, FrameDecoder, FrameEncoder};
use std::fmt::Debug;
use std::time::Duration;

pub struct Connection {
    close_tx: broadcast::Sender<()>,
    cipher_change_tx: mpsc::Sender<CipherMode>,
    inbound_frame_rx: mpsc::Receiver<Frame>,
    outbound_frame_tx: mpsc::Sender<Frame>,
    handle: JoinHandle<()>,
}

impl Connection {
    pub fn start<Read>(cipher_pair: (CipherMode, CipherMode), stream: Read) -> Connection
    where
        Read: AsyncRead + AsyncWrite + Unpin + Send + Debug + 'static,
    {
        let (close_tx, mut close_rx) = broadcast::channel::<()>(1);
        let (cipher_change_tx, mut cipher_change_rx) = mpsc::channel::<CipherMode>(1);
        let (outbound_frame_tx, mut outbound_frame_rx) = mpsc::channel::<Frame>(10);
        let (inbound_frame_tx, inbound_frame_rx) = mpsc::channel::<Frame>(10);
        let (inbound_cipher, outbound_cipher) = cipher_pair;

        let handle = tokio::spawn(async move {
            let (mut stream_reader, mut stream_writer) = split(stream);
            let mut frame_reader =
                FramedRead::new(&mut stream_reader, FrameDecoder::new(inbound_cipher, false));
            let mut frame_writer =
                FramedWrite::new(&mut stream_writer, FrameEncoder::new(outbound_cipher, true));

            loop {
                tokio::select! {
                    cipher = cipher_change_rx.recv() => {
                        match cipher {
                            Some(cipher) => {
                               info!("Cipher change");
                                let new_cipher = cipher;
                                frame_reader.decoder_mut().set_cipher_mode(new_cipher.clone());
                                frame_writer.encoder_mut().set_cipher_mode(new_cipher.clone());
                            }
                            None => {
                              info!("Cipher channel closed");
                              break;
                            }
                        }
                    }
                    inbound_frame = frame_reader.next() => {
                        match inbound_frame {
                            Some(Ok(frame)) => {
                                let _ = inbound_frame_tx.send(frame).await;
                            },
                            Some(Err(error)) => {
                                tracing::error!(error = %error, "error while reading frame");
                                break;
                            }
                            None => {
                                info!("connection was closed");
                                break;
                            }
                        }
                    }
                    outbound_frame = outbound_frame_rx.recv() => {
                        match outbound_frame {
                            Some(frame) => {
                                let _ = frame_writer.send(frame).await;
                            }
                            None => break
                        }
                    }
                    _ = close_rx.recv() => {
                        info!("received close signal");
                        break;
                    }
                }
            }
        });

        Connection {
            close_tx,
            handle,
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
