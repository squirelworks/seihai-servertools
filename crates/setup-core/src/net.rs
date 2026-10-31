//! Network helpers for the setup wizard.
//!
//! [`test_connection`] makes one TCP connection to the address the owner
//! typed and reports open / refused / timed out. It never connects anywhere
//! else, and nothing in this crate makes a call the owner did not ask for
//! (no telemetry, no update checks).
//!
//! **Planned** (`docs/NEXT.md`): `interfaces()` for the "Listen on" list
//! (needs a small MIT/Apache crate such as `if-addrs`), and opt-in UPnP
//! (off by default, explained in the wizard, never needs admin rights).
//!
//! Tests only touch `127.0.0.1`.

use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// What a test connection found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reach {
    /// Something accepted the connection.
    Open,
    /// The host answered that nothing listens there.
    Refused,
    /// No answer in time (a firewall, a wrong address, a host that is down).
    TimedOut,
    /// The name did not resolve, or another error.
    Error(String),
}

impl std::fmt::Display for Reach {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reach::Open => write!(f, "open"),
            Reach::Refused => write!(f, "refused (nothing listens there)"),
            Reach::TimedOut => write!(f, "timed out (no answer)"),
            Reach::Error(e) => write!(f, "error: {e}"),
        }
    }
}

/// Connect once to `host:port` and close again.
pub fn test_connection(host: &str, port: u16, timeout: Duration) -> Reach {
    let addrs = match (host, port).to_socket_addrs() {
        Ok(a) => a.collect::<Vec<_>>(),
        Err(e) => return Reach::Error(e.to_string()),
    };
    let mut last = Reach::Error(format!("{host} did not resolve"));
    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(_) => return Reach::Open,
            Err(e) if e.kind() == io::ErrorKind::ConnectionRefused => last = Reach::Refused,
            Err(e)
                if e.kind() == io::ErrorKind::TimedOut || e.kind() == io::ErrorKind::WouldBlock =>
            {
                last = Reach::TimedOut
            }
            Err(e) => last = Reach::Error(e.to_string()),
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn open_and_refused_on_loopback() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        assert_eq!(
            test_connection("127.0.0.1", port, Duration::from_secs(2)),
            Reach::Open
        );
        drop(l);
        let r = test_connection("127.0.0.1", port, Duration::from_secs(2));
        // Windows may report a refusal as a timeout on a closed loopback port.
        assert!(matches!(r, Reach::Refused | Reach::TimedOut), "{r:?}");
    }
}
