use std::marker::PhantomData;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;

use async_trait::async_trait;
use tokio::io;
use tokio::net::TcpListener;
use tracing::{info, info_span};

use crate::net::{CipherMode, Connection};

#[async_trait]
pub trait ConnectionHandler<Ctx>: Default + Send + 'static
where
    Ctx: Clone + Send + 'static,
{
    fn description() -> &'static str;

    async fn run(&mut self, connection: &mut Connection, context: Ctx);
}

pub struct TcpServer<Ctx, Handler>
where
    Ctx: Clone + Send + 'static,
    Handler: ConnectionHandler<Ctx>,
{
    bind_address: Vec<SocketAddr>,
    cipher_pair: (CipherMode, CipherMode),
    context: Ctx,
    _handler: PhantomData<Handler>,
}

impl<Ctx: 'static, Handler> TcpServer<Ctx, Handler>
where
    Ctx: Clone + Send + 'static,
    Handler: ConnectionHandler<Ctx>,
{
    pub fn new<Addrs>(
        bind_addresses: Addrs,
        cipher_pair: (CipherMode, CipherMode),
        context: Ctx,
    ) -> Self
    where
        Addrs: ToSocketAddrs,
    {
        Self {
            bind_address: bind_addresses.to_socket_addrs().unwrap().collect(),
            cipher_pair,
            context,
            _handler: PhantomData,
        }
    }

    pub async fn run(&mut self) -> crate::Result<()> {
        info!("Starting {} server", Handler::description());

        let mut listener = TcpListener::bind(&*self.bind_address).await.map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Error binding to <{:#?}>: {}", &self.bind_address, e),
            )
        })?;

        info!("Now waiting for connections on <{:#?}>", &self.bind_address);

        loop {
            match self.accept(&mut listener).await {
                Ok(mut connection) => {
                    let ctx = self.context.clone();

                    tokio::spawn(async move {
                        let mut handler = Handler::default();

                        handler.run(&mut connection, ctx).await
                    });
                }
                Err(e) => {
                    info!("Error while accepting connection");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn accept(&mut self, listener: &mut TcpListener) -> crate::Result<Connection> {
        let mut retry_count: u8 = 1;
        let mut backoff: u64 = 500;

        loop {
            match listener.accept().await {
                Ok((stream, _)) => return Ok(Connection::start(self.cipher_pair.clone(), stream)),
                Err(_) if retry_count < 3 => {
                    tokio::time::sleep(Duration::from_millis(backoff)).await;

                    retry_count += 1;
                    backoff *= 2;
                }
                Err(e) => return Err(Box::new(e)),
            };
        }
    }
}
