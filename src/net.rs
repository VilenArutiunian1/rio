use libc::{
    c_int, in6_addr, in_addr, sa_family_t, sockaddr, sockaddr_in, sockaddr_in6, sockaddr_storage,
    socklen_t, AF_INET, AF_INET6, SOCK_CLOEXEC, SOCK_NONBLOCK,
};
use std::{
    io,
    mem::size_of,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

use crate::syscall;

pub(crate) fn create_new_socket(domain: c_int, socket_type: c_int) -> io::Result<c_int> {
    let socket_type = socket_type | SOCK_NONBLOCK | SOCK_CLOEXEC;
    syscall!(socket(domain, socket_type, 0))
}

/// Used in converting Rust level SocketAddr* types into their system representation
#[repr(C)]
pub(crate) union SocketAddrCRepr {
    v4: sockaddr_in,
    v6: sockaddr_in6,
}

impl SocketAddrCRepr {
    pub(crate) fn as_ptr(&self) -> *const sockaddr {
        self as *const _ as *const sockaddr
    }
}

/// Converts a Rust `SocketAddr` into the system representation.
pub(crate) fn socket_addr(addr: &SocketAddr) -> (SocketAddrCRepr, socklen_t) {
    match addr {
        SocketAddr::V4(ref addr) => {
            let sin_addr = in_addr {
                s_addr: u32::from_ne_bytes(addr.ip().octets()),
            };

            let sockaddr_in = sockaddr_in {
                sin_family: AF_INET as sa_family_t,
                sin_port: addr.port().to_be(),
                sin_addr,
                sin_zero: [0; 8],
            };

            let sockaddr = SocketAddrCRepr { v4: sockaddr_in };
            let socklen = size_of::<sockaddr_in>() as socklen_t;
            (sockaddr, socklen)
        }
        SocketAddr::V6(ref addr) => {
            let sockaddr_in6 = sockaddr_in6 {
                sin6_family: AF_INET6 as sa_family_t,
                sin6_port: addr.port().to_be(),
                sin6_addr: in6_addr {
                    s6_addr: addr.ip().octets(),
                },
                sin6_flowinfo: addr.flowinfo(),
                sin6_scope_id: addr.scope_id(),
            };

            let sockaddr = SocketAddrCRepr { v6: sockaddr_in6 };
            let socklen = size_of::<sockaddr_in6>() as socklen_t;
            (sockaddr, socklen)
        }
    }
}

pub(crate) unsafe fn to_socket_addr(storage: *const sockaddr_storage) -> io::Result<SocketAddr> {
    match (*storage).ss_family as c_int {
        AF_INET => {
            let addr: &sockaddr_in = &*(storage as *const sockaddr_in);
            let ip = Ipv4Addr::from(addr.sin_addr.s_addr.to_ne_bytes());
            let port = u16::from_be(addr.sin_port);
            Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
        }
        AF_INET6 => {
            let addr: &sockaddr_in6 = &*(storage as *const sockaddr_in6);
            let ip = Ipv6Addr::from(addr.sin6_addr.s6_addr);
            let port = u16::from_be(addr.sin6_port);
            Ok(SocketAddr::V6(SocketAddrV6::new(
                ip,
                port,
                addr.sin6_flowinfo,
                addr.sin6_scope_id,
            )))
        }
        _ => Err(io::ErrorKind::InvalidInput.into()),
    }
}
