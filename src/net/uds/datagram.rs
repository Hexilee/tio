use super::SocketAddr;
use crate::net::poll::Watcher;
use futures::future;
use mio::net;
use std::io;
use std::net::Shutdown;
use std::os::unix::io::{AsRawFd, RawFd};
use std::os::unix::net::UnixDatagram as StdDatagram;
use std::path::Path;
use std::sync::Arc;

/// A Unix datagram socket.
///
/// After creating a `UnixDatagram` by [`bind`]ing it to a path, data can be [sent to] and
/// [received from] any other socket address.
///
/// This type is an async version of [`std::os::unix::net::UnixDatagram`].
///
/// [`std::os::unix::net::UnixDatagram`]:
/// https://doc.rust-lang.org/std/os/unix/net/struct.UnixDatagram.html
/// [`bind`]: #method.bind
/// [received from]: #method.recv_from
/// [sent to]: #method.send_to
///
/// ## Examples
///
/// ```no_run
/// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
/// #
/// use tio::net::UnixDatagram;
///
/// let socket = UnixDatagram::bind("/tmp/socket1")?;
/// socket.send_to(b"hello world", "/tmp/socket2").await?;
///
/// let mut buf = vec![0u8; 1024];
/// let (n, peer) = socket.recv_from(&mut buf).await?;
/// #
/// # Ok(()) }) }
/// ```
#[derive(Debug, Clone)]
pub struct UnixDatagram(Arc<Watcher<net::UnixDatagram>>);

impl UnixDatagram {
    fn new(datagram: net::UnixDatagram) -> UnixDatagram {
        Self(Arc::new(Watcher::new(datagram)))
    }

    /// Creates a Unix datagram socket bound to the given path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::bind("/tmp/socket")?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<UnixDatagram> {
        let datagram = net::UnixDatagram::bind(path)?;
        Ok(UnixDatagram::new(datagram))
    }

    /// Creates a Unix datagram which is not bound to any address.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn unbound() -> io::Result<UnixDatagram> {
        let socket = net::UnixDatagram::unbound()?;
        Ok(UnixDatagram::new(socket))
    }

    /// Creates an unnamed pair of connected sockets.
    ///
    /// Returns two sockets which are connected to each other.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let (socket1, socket2) = UnixDatagram::pair()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn pair() -> io::Result<(UnixDatagram, UnixDatagram)> {
        let (a, b) = net::UnixDatagram::pair()?;
        let a = UnixDatagram::new(a);
        let b = UnixDatagram::new(b);
        Ok((a, b))
    }

    /// Connects the socket to the specified address.
    ///
    /// The [`send`] method may be used to send data to the specified address. [`recv`] and
    /// [`recv_from`] will only receive data from that address.
    ///
    /// [`send`]: #method.send
    /// [`recv`]: #method.recv
    /// [`recv_from`]: #method.recv_from
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.connect("/tmp/socket")?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn connect<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let p = path.as_ref();
        self.0.connect(p)
    }

    /// Returns the address of this socket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::bind("/tmp/socket")?;
    /// let addr = socket.local_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }

    /// Returns the address of this socket's peer.
    ///
    /// The [`connect`] method will connect the socket to a peer.
    ///
    /// [`connect`]: #method.connect
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.connect("/tmp/socket")?;
    /// let peer = socket.peer_addr()?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.0.peer_addr()
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read and the address from where the data came.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// let mut buf = vec![0; 1024];
    /// let (n, peer) = socket.recv_from(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        future::poll_fn(|cx| self.0.poll_read_with(cx, |inner| inner.recv_from(buf)))
            .await
    }

    /// Receives data from the socket.
    ///
    /// On success, returns the number of bytes read.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::bind("/tmp/socket")?;
    /// let mut buf = vec![0; 1024];
    /// let n = socket.recv(&mut buf).await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        future::poll_fn(|cx| self.0.poll_read_with(cx, |inner| inner.recv(buf))).await
    }

    /// Sends data on the socket to the specified address.
    ///
    /// On success, returns the number of bytes written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.send_to(b"hello world", "/tmp/socket").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn send_to<P: AsRef<Path>>(
        &self,
        buf: &[u8],
        path: P,
    ) -> io::Result<usize> {
        future::poll_fn(|cx| {
            self.0
                .poll_write_with(cx, |inner| inner.send_to(buf, path.as_ref()))
        })
        .await
    }

    /// Sends data on the socket to the socket's peer.
    ///
    /// On success, returns the number of bytes written.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.connect("/tmp/socket")?;
    /// socket.send(b"hello world").await?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub async fn send(&self, buf: &[u8]) -> io::Result<usize> {
        future::poll_fn(|cx| self.0.poll_write_with(cx, |inner| inner.send(buf))).await
    }

    /// Shut down the read, write, or both halves of this connection.
    ///
    /// This function will cause all pending and future I/O calls on the specified portions to
    /// immediately return with an appropriate value (see the documentation of [`Shutdown`]).
    ///
    /// [`Shutdown`]: https://doc.rust-lang.org/std/net/enum.Shutdown.html
    ///
    /// ## Examples
    ///
    /// ```no_run
    /// # fn main() -> std::io::Result<()> { tio::task::block_on(async {
    /// #
    /// use tio::net::UnixDatagram;
    /// use std::net::Shutdown;
    ///
    /// let socket = UnixDatagram::unbound()?;
    /// socket.shutdown(Shutdown::Both)?;
    /// #
    /// # Ok(()) }) }
    /// ```
    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.0.shutdown(how)
    }
}

impl From<StdDatagram> for UnixDatagram {
    /// Converts a `std::os::unix::net::UnixDatagram` into its asynchronous equivalent.
    ///
    /// # Notes
    ///
    /// The caller is responsible for ensuring that the listener is in
    /// non-blocking mode.
    fn from(datagram: StdDatagram) -> UnixDatagram {
        let mio_datagram = net::UnixDatagram::from_std(datagram);
        Self::new(mio_datagram)
    }
}

impl AsRawFd for UnixDatagram {
    /// Share raw fd of `UnixDatagram`.
    ///
    /// # Notes
    ///
    /// The caller is responsible for never closing this fd.
    fn as_raw_fd(&self) -> RawFd {
        self.0.as_raw_fd()
    }
}

#[cfg(test)]
mod tests {
    use super::UnixDatagram;
    use crate::task::{block_on, spawn};
    use std::io;
    use std::net::Shutdown;
    use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd};
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn random_path() -> io::Result<PathBuf> {
        Ok(NamedTempFile::new()?.path().to_path_buf())
    }

    const DATA: &[u8] = b"
    If you prick us, do we not bleed?
    If you tickle us, do we not laugh?
    If you poison us, do we not die?
    And if you wrong us, shall we not revenge?
    ";

    fn one() -> io::Result<UnixDatagram> {
        UnixDatagram::bind(random_path()?)
    }

    fn server() -> io::Result<PathBuf> {
        let path = random_path()?;
        let socket = UnixDatagram::bind(path.as_path())?;
        spawn(async move {
            let mut data = [0; 1024];
            while let Ok((size, addr)) = socket.recv_from(&mut data).await {
                assert_eq!(DATA, &data[..size]);
                if let Some(path) = addr.as_pathname() {
                    socket.send_to(&data[..size], path).await.unwrap();
                }
            }
        });
        Ok(path)
    }

    #[test]
    fn echo() -> io::Result<()> {
        block_on(async {
            let mut data = [0; 1024];
            let socket = one()?;
            let server_addr = server()?;
            socket.connect(server_addr)?;
            socket.send(DATA).await?;
            let size = socket.recv(&mut data).await?;
            assert_eq!(DATA, &data[..size]);
            Ok(())
        })
    }

    #[test]
    fn from_std() -> io::Result<()> {
        block_on(async {
            let mut data = [0; 1024];
            let server_addr = server()?;
            let raw_socket = std::os::unix::net::UnixDatagram::bind(random_path()?)?;
            raw_socket.set_nonblocking(true)?;
            raw_socket.connect(server_addr)?;
            let socket: UnixDatagram = raw_socket.into();
            socket.send(DATA).await?;
            let size = socket.recv(&mut data).await?;
            assert_eq!(DATA, &data[..size]);
            Ok(())
        })
    }

    #[test]
    fn as_raw_fd() -> io::Result<()> {
        block_on(async {
            let mut data = [0; 1024];
            let server_addr = server()?;
            let socket = UnixDatagram::bind(random_path()?)?;
            let fd = socket.as_raw_fd();
            let raw_socket =
                unsafe { std::os::unix::net::UnixDatagram::from_raw_fd(fd) };
            raw_socket.set_nonblocking(false)?;
            raw_socket.connect(server_addr)?;
            raw_socket.send(DATA)?;
            let size = raw_socket.recv(&mut data)?;
            assert_eq!(DATA, &data[..size]);
            raw_socket.into_raw_fd(); // avoid fd closed when raw_socket is dropped
            Ok(())
        })
    }

    #[test]
    fn unbound() -> io::Result<()> {
        block_on(async {
            let socket = UnixDatagram::unbound()?;
            let server_addr = server()?;
            socket.connect(server_addr.as_path())?;
            socket.send(DATA).await?;
            assert!(socket.local_addr()?.is_unnamed());
            assert_eq!(
                Some(server_addr.as_path()),
                socket.peer_addr()?.as_pathname()
            );
            Ok(())
        })
    }

    #[test]
    fn shutdown() -> io::Result<()> {
        block_on(async {
            let socket = UnixDatagram::unbound()?;
            let server_addr = server()?;
            socket.connect(server_addr.as_path())?;
            socket.shutdown(Shutdown::Both)?;
            assert!(socket.send(DATA).await.is_err());
            Ok(())
        })
    }

    #[test]
    fn pair() -> io::Result<()> {
        use crate::task::spawn;
        block_on(async {
            let (s1, s2) = UnixDatagram::pair()?;
            spawn(async move {
                let mut data = [0; 1024];
                let size = s2.recv(&mut data).await.unwrap();
                assert_eq!(DATA, &data[..size]);
                s2.send(DATA).await.unwrap();
            });
            s1.send(DATA).await?;
            let mut data = [0; 1024];
            let size = s1.recv(&mut data).await?;
            assert_eq!(DATA, &data[..size]);
            Ok(())
        })
    }
}
