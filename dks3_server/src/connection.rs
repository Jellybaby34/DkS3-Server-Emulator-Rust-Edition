use std::iter::Iterator;

use futures::SinkExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, FramedWrite};

use dks3_proto::frame::{Frame, FrameDecoder, FrameEncoder};

pub struct Connection {
    close_tx: broadcast::Sender<()>,
    inbound_frame_rx: mpsc::Receiver<Frame>,
    outbound_frame_tx: mpsc::Sender<Frame>,
}

impl Connection {
    pub fn start<Read, Write>(reader: Read, writer: Write) -> Connection
    where
        Read: AsyncRead + Unpin + Send + 'static,
        Write: AsyncWrite + Unpin + Send + 'static,
    {
        let (close_tx, mut close_rx) = broadcast::channel::<()>(1);
        let (outbound_frame_tx, mut outbound_frame_rx) = mpsc::channel::<Frame>(10);
        let (inbound_frame_tx, inbound_frame_rx) = mpsc::channel::<Frame>(10);

        let mut frame_reader = FramedRead::new(reader, FrameDecoder::new(false));
        let mut frame_writer = FramedWrite::new(writer, FrameEncoder::new(true));

        let io_task = tokio::spawn(async move {
            loop {
                tokio::select! {
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
            inbound_frame_rx,
            outbound_frame_tx,
        }
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
