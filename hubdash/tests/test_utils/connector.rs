//! Turmoil TCP connector wiring for Axum/Hyper.
//!
//! Wraps `turmoil::net::{TcpListener, TcpStream}` behind the traits needed
//! by both Axum's server (`axum::serve::Listener`) and hyper's legacy client
//! (`hyper_util::client::legacy::connect::Connection`).

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

/// A turmoil-backed TCP listener usable as an Axum `serve::Listener`.
pub struct SimListener(pub TcpListener);

/// A turmoil-backed TCP stream.
pub struct SimStream(pub TcpStream);

/// Future type returned by the connector service.
pub type ConnectFuture = Pin<Box<dyn Future<Output = io::Result<TokioIo<SimStream>>> + Send>>;

/// A zero-size connector that routes connections through turmoil's TCP stack.
///
/// Implements `tower::Service<Uri>` so it can be passed directly to
/// `hyper_util::client::legacy::Client::builder(...).build(SimConnector)`.
#[derive(Clone, Copy)]
pub struct SimConnector;

impl Service<Uri> for SimConnector {
    type Response = TokioIo<SimStream>;
    type Error = io::Error;
    type Future = ConnectFuture;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        Box::pin(async move {
            let host = uri.host().unwrap_or("localhost");
            let port = uri.port_u16().unwrap_or_else(|| match uri.scheme_str() {
                Some("https") => 443,
                _ => 80,
            });
            let stream = TcpStream::connect(format!("{host}:{port}")).await?;
            Ok(TokioIo::new(SimStream(stream)))
        })
    }
}

/// Binds a `SimListener` on the given address inside the turmoil simulation.
pub async fn sim_listen(addr: SocketAddr) -> io::Result<SimListener> {
    Ok(SimListener(TcpListener::bind(addr).await?))
}

// ── axum::serve::Listener ───────────────────────────────────────────────────

impl axum::serve::Listener for SimListener {
    type Io = SimStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        self.0
            .accept()
            .await
            .map(|(s, a)| (SimStream(s), a))
            .unwrap()
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }
}

// ── AsyncRead / AsyncWrite for SimStream ────────────────────────────────────

impl AsyncRead for SimStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for SimStream {
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

// ── hyper legacy-client connection metadata ─────────────────────────────────

impl hyper_util::client::legacy::connect::Connection for SimStream {
    fn connected(&self) -> hyper_util::client::legacy::connect::Connected {
        hyper_util::client::legacy::connect::Connected::new()
    }
}
