pub mod epoll {
    pub use epoll_rs::{Epoll, Event, Interest, Token};
}
pub mod tcp;
pub mod net;

#[allow(unused_macros)]

#[macro_export]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}