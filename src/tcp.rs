use std::{
    io::{self, IoSlice, IoSliceMut, Read, Write},
    mem::{size_of, MaybeUninit},
    net::{self, Shutdown, SocketAddr},
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, RawFd},
};

use libc::{
    c_int, c_void, sockaddr_storage, socklen_t, AF_INET, AF_INET6, EINPROGRESS, SOCK_CLOEXEC,
    SOCK_NONBLOCK, SOCK_STREAM, SOL_SOCKET, SO_REUSEADDR,
};

use crate::syscall;

use super::net::{create_new_socket, socket_addr, to_socket_addr};

pub(crate) fn new_for_addr(addr: SocketAddr) -> io::Result<c_int> {
    let domain = match addr {
        SocketAddr::V4(_) => AF_INET,
        SocketAddr::V6(_) => AF_INET6,
    };
    create_new_socket(domain, SOCK_STREAM)
}

pub struct TcpListener {
    inner: net::TcpListener,
}

impl TcpListener {
    pub fn bind(addr: SocketAddr) -> io::Result<TcpListener> {
        let socket = new_for_addr(addr)?;

        let listener = unsafe { TcpListener::from_raw_fd(socket) };

        let val: c_int = 1;
        syscall!(setsockopt(
            listener.as_raw_fd(),
            SOL_SOCKET,
            SO_REUSEADDR,
            &val as *const c_int as *const c_void,
            size_of::<c_int>() as socklen_t,
        ))?;

        let (raw_addr, raw_addr_length) = socket_addr(&addr);
        syscall!(bind(
            listener.as_raw_fd(),
            raw_addr.as_ptr(),
            raw_addr_length
        ))?;

        syscall!(listen(listener.as_raw_fd(), 1024))?;

        Ok(listener)
    }

    pub fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let mut addr = MaybeUninit::uninit();
        let mut length = size_of::<sockaddr_storage>() as socklen_t;
        let stream = {
            syscall!(accept4(
                self.as_raw_fd(),
                addr.as_mut_ptr() as *mut _,
                &mut length,
                SOCK_CLOEXEC | SOCK_NONBLOCK,
            ))
            .map(|socket| unsafe { net::TcpStream::from_raw_fd(socket) })
        }?;
        match unsafe { to_socket_addr(addr.as_ptr()) } {
            Ok(addr) => Ok((TcpStream::from_std(stream), addr)),
            Err(e) => Err(e),
        }
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.inner.set_ttl(ttl)
    }

    pub fn ttl(&self) -> io::Result<u32> {
        self.inner.ttl()
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.inner.take_error()
    }

    pub fn from_std(listener: net::TcpListener) -> TcpListener {
        Self::from(listener)
    }
}

impl From<net::TcpListener> for TcpListener {
    fn from(l: net::TcpListener) -> Self {
        TcpListener { inner: l }
    }
}

impl IntoRawFd for TcpListener {
    fn into_raw_fd(self) -> RawFd {
        self.inner.into_raw_fd()
    }
}

impl AsRawFd for TcpListener {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl FromRawFd for TcpListener {
    unsafe fn from_raw_fd(fd: RawFd) -> TcpListener {
        TcpListener {
            inner: net::TcpListener::from_raw_fd(fd),
        }
    }
}

pub struct TcpStream {
    inner: net::TcpStream,
}

impl TcpStream {
    pub fn connect(addr: SocketAddr) -> io::Result<TcpStream> {
        let socket = new_for_addr(addr)?;
        let stream = unsafe { TcpStream::from_raw_fd(socket) };
        let (raw_addr, raw_addr_length) = socket_addr(&addr);

        match syscall!(connect(
            socket.as_raw_fd(),
            raw_addr.as_ptr(),
            raw_addr_length
        )) {
            Err(err) if err.raw_os_error() != Some(EINPROGRESS) => Err(err),
            _ => Ok(()),
        }?;
        Ok(stream)
    }

    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.inner.peer_addr()
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }

    pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
        self.inner.shutdown(how)
    }

    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        self.inner.set_nodelay(nodelay)
    }

    pub fn nodelay(&self) -> io::Result<bool> {
        self.inner.nodelay()
    }

    pub fn set_ttl(&self, ttl: u32) -> io::Result<()> {
        self.inner.set_ttl(ttl)
    }

    pub fn ttl(&self) -> io::Result<u32> {
        self.inner.ttl()
    }

    pub fn take_error(&self) -> io::Result<Option<io::Error>> {
        self.inner.take_error()
    }

    pub fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.peek(buf)
    }

    pub fn from_std(stream: net::TcpStream) -> TcpStream {
        Self::from(stream)
    }
}

impl Read for TcpStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        self.inner.read_vectored(bufs)
    }
}

impl Write for TcpStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.inner.write_vectored(bufs)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl From<net::TcpStream> for TcpStream {
    fn from(s: net::TcpStream) -> Self {
        TcpStream { inner: s }
    }
}

impl IntoRawFd for TcpStream {
    fn into_raw_fd(self) -> RawFd {
        self.inner.into_raw_fd()
    }
}

impl AsRawFd for TcpStream {
    fn as_raw_fd(&self) -> RawFd {
        self.inner.as_raw_fd()
    }
}

impl FromRawFd for TcpStream {
    unsafe fn from_raw_fd(fd: RawFd) -> TcpStream {
        TcpStream::from_std(FromRawFd::from_raw_fd(fd))
    }
}
