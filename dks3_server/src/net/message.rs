use bytes::BytesMut;
use prost::Message;

use dks3_proto::frame::Frame;

use crate::net::Connection;

pub async fn write_data(
    conn: &mut Connection,
    data: &[u8],
    global_counter: u16,
    counter: u32
) {
    let data_buffer = bytes::BytesMut::from(data);
    
    conn.write_frame(Frame::new(global_counter, counter, data_buffer))
    .await;
}

pub async fn write_message<M: Message>(
    conn: &mut Connection,
    message: M,
    global_counter: u16,
    counter: u32,
) {
    let message_len = message.encoded_len();
    let mut message_data = BytesMut::with_capacity(message_len);

    message.encode(&mut message_data);

    conn.write_frame(Frame::new(global_counter, counter, message_data))
        .await;
}

pub async fn read_message<M: Message + Default>(conn: &mut Connection) -> (M, u16, u32) {
    let frame = conn.read_frame().await.unwrap();
    let msg = M::decode(frame.data).unwrap(); // TODO: handle error

    (msg, frame.global_counter, frame.counter)
}
