//! Frame-level asynchronous transport over TCP.

use std::future::Future;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpStream, ToSocketAddrs};

use crate::{encode_stream_frame, Frame, FrameDecoder, FrameError};

/// Failure to send or receive a frame.
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    /// An underlying socket operation failed.
    #[error("transport I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// Host identity or viewing permission was rejected; never retry as plaintext.
    #[error("secure connection rejected: {0}")]
    Authentication(String),
    /// A frame could not be encoded or decoded.
    #[error("invalid frame: {0}")]
    Frame(#[from] FrameError),
    /// The peer closed at a frame boundary.
    #[error("peer closed the transport")]
    Closed,
    /// An operation exceeded its configured deadline.
    #[error("{0} timed out")]
    Timeout(&'static str),
}

/// An ordered frame transport, usable through `Box<dyn Transport>`.
#[async_trait]
pub trait Transport: Send {
    /// Send one frame, preserving its kind and opaque payload.
    ///
    /// `TcpTransport` copies the payload into a newly allocated encoded buffer
    /// on every call. Its default 10-second write timeout means a max-size
    /// 16 MiB frame needs roughly 1.6 MiB/s of sustained throughput to complete.
    /// After cancellation or a write timeout, drop the connection: a partial
    /// frame may already have been written.
    async fn send(&mut self, kind: u8, payload: &[u8]) -> Result<(), TransportError>;

    /// Receive one frame. Cancellation retains bytes already read for retry.
    ///
    /// On `Frame` or `Closed`, the TCP receive path keeps returning that error:
    /// malformed input poisons the decoder, while EOF persists on the socket
    /// (partial input at EOF repeatedly returns `Frame(Truncated)`). Drop the
    /// connection after these errors.
    async fn recv(&mut self) -> Result<Frame, TransportError>;
}

/// TCP transport with an incremental decoder and operation deadlines.
///
/// Connections enable TCP_NODELAY. Connect defaults to five seconds, writes to
/// ten seconds, and reads have no deadline. Sending allocates a fresh encoded
/// buffer and copies the payload on each call. With the default write timeout,
/// a max-size 16 MiB frame requires about 1.6 MiB/s sustained throughput.
/// Drop and reconnect after malformed frames, EOF, I/O failures, or interrupted
/// writes; no automatic replay is performed.
pub struct FramedTransport<S> {
    stream: S,
    decoder: FrameDecoder,
    read_buffer: Box<[u8; 64 * 1024]>,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
}

/// Legacy plaintext TCP specialization. Secure channels use FramedTransport
/// with an already authenticated TLS stream; framing/limits stay identical.
pub type TcpTransport = FramedTransport<TcpStream>;

impl<S: AsyncRead + AsyncWrite + Unpin + Send> FramedTransport<S> {
    /// Wrap an established byte stream. TLS authentication belongs before this.
    pub fn from_stream(stream: S) -> Self {
        Self {
            stream,
            decoder: FrameDecoder::new(),
            read_buffer: Box::new([0; 64 * 1024]),
            read_timeout: None,
            write_timeout: Some(Duration::from_secs(10)),
        }
    }
}
impl FramedTransport<TcpStream> {
    /// Wrap a connected TCP stream and enable TCP_NODELAY.
    pub fn new(stream: TcpStream) -> Result<Self, TransportError> {
        stream.set_nodelay(true)?;
        Ok(Self::from_stream(stream))
    }
    /// Resolve and connect within five seconds, then enable TCP_NODELAY.
    pub async fn connect(addr: impl ToSocketAddrs) -> Result<Self, TransportError> {
        Self::connect_with_timeout(addr, Duration::from_secs(5)).await
    }

    /// Resolve and connect within the supplied deadline, then enable TCP_NODELAY.
    pub async fn connect_with_timeout(
        addr: impl ToSocketAddrs,
        timeout: Duration,
    ) -> Result<Self, TransportError> {
        let stream = with_timeout(Some(timeout), "connect", async {
            Ok(TcpStream::connect(addr).await?)
        })
        .await?;
        Self::new(stream)
    }
}
impl<S: AsyncRead + AsyncWrite + Unpin + Send> FramedTransport<S> {
    /// Set a deadline for the entire next receive, or `None` to wait indefinitely.
    /// A read timeout retains partial frame bytes and permits retrying `recv`.
    pub fn set_read_timeout(&mut self, timeout: Option<Duration>) {
        self.read_timeout = timeout;
    }

    /// Set a deadline for writing and flushing a frame, or `None` to disable it.
    /// Drop the connection after a write timeout because output may be partial.
    pub fn set_write_timeout(&mut self, timeout: Option<Duration>) {
        self.write_timeout = timeout;
    }

    async fn receive_frame(&mut self) -> Result<Frame, TransportError> {
        loop {
            if let Some(frame) = self.decoder.next_frame()? {
                return Ok(frame);
            }
            let count = self.stream.read(&mut self.read_buffer[..]).await?;
            if count == 0 {
                return if self.decoder.buffered_len() == 0 {
                    Err(TransportError::Closed)
                } else {
                    Err(FrameError::Truncated.into())
                };
            }
            self.decoder.push(&self.read_buffer[..count]);
        }
    }
}

#[async_trait]
impl<S: AsyncRead + AsyncWrite + Unpin + Send> Transport for FramedTransport<S> {
    /// Encode and send one frame, allocating a new buffer and copying `payload`.
    /// With the default 10-second write timeout, a max-size 16 MiB frame needs
    /// about 1.6 MiB/s of sustained throughput to complete.
    async fn send(&mut self, kind: u8, payload: &[u8]) -> Result<(), TransportError> {
        let bytes = encode_stream_frame(kind, payload)?;
        with_timeout(self.write_timeout, "write", async {
            self.stream.write_all(&bytes).await?;
            self.stream.flush().await?;
            Ok(())
        })
        .await
    }

    async fn recv(&mut self) -> Result<Frame, TransportError> {
        with_timeout(self.read_timeout, "read", self.receive_frame()).await
    }
}

async fn with_timeout<T>(
    duration: Option<Duration>,
    operation: &'static str,
    future: impl Future<Output = Result<T, TransportError>>,
) -> Result<T, TransportError> {
    match duration {
        Some(duration) => tokio::time::timeout(duration, future)
            .await
            .map_err(|_| TransportError::Timeout(operation))?,
        None => future.await,
    }
}
