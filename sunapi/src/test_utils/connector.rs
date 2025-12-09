use std::{
    future::Future,
    io,
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use http::Uri;
use hyper_util::rt::TokioIo;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tower::Service;
use turmoil::net::{TcpListener, TcpStream};

pub struct Listener(pub TcpListener);
pub struct Stream(pub TcpStream);

pub async fn listen(addr: SocketAddr) -> io::Result<Listener> {
    Ok(Listener(TcpListener::bind(addr).await?))
}

pub type ConnectFuture = Pin<Box<dyn Future<Output = io::Result<TokioIo<Stream>>> + Send>>;

pub fn connector()
-> impl Service<Uri, Response = TokioIo<Stream>, Error = io::Error, Future = ConnectFuture> + Clone
{
    let connector = |uri: Uri| {
        Box::pin(async move {
            let stream = TcpStream::connect(uri.authority().unwrap().as_str()).await?;
            Ok(TokioIo::new(Stream(stream)))
        }) as ConnectFuture
    };
    tower::service_fn(connector)
}

impl axum::serve::Listener for Listener {
    type Io = Stream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        self.0.accept().await.map(|(x, y)| (Stream(x), y)).unwrap()
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }
}

impl AsyncRead for Stream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for Stream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

impl hyper_util::client::legacy::connect::Connection for Stream {
    fn connected(&self) -> hyper_util::client::legacy::connect::Connected {
        hyper_util::client::legacy::connect::Connected::new()
    }
}
