//! Network helpers for the setup wizard.
//!
//! [`test_connection`] makes one TCP connection to the address the owner
//! typed and reports open / refused / timed out. It never connects anywhere
//! else, and nothing in this crate makes a call the owner did not ask for
//! (no telemetry, no update checks).
//!
//! [`interfaces`] lists this machine's addresses for the "Listen on" pick
//! lists. It reads the local interface table (`if-addrs`, MIT); it sends
//! nothing on the network.
//!
//! **Planned** (`docs/NEXT.md`): opt-in UPnP (off by default, explained in
//! the wizard, never needs admin rights).
//!
//! Tests only touch `127.0.0.1`.

use std::io;
use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

/// One address of this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    /// The interface's name (`lo`, `eth0`, `Ethernet 2`).
    pub name: String,
    pub addr: IpAddr,
    /// `127.0.0.1`, `::1` and the rest of the loopback ranges: only this
    /// machine can connect there.
    pub loopback: bool,
}

impl Interface {
    /// `addr (name)`, plus "this machine only" for loopback.
    pub fn label(&self) -> String {
        if self.loopback {
            format!("{} ({}, this machine only)", self.addr, self.name)
        } else {
            format!("{} ({})", self.addr, self.name)
        }
    }
}

/// This machine's addresses: IPv4 first, then IPv6, loopback last in each,
/// then by address. IPv6 link-local addresses (`fe80::`) are left out: a
/// server cannot listen on one without a scope id, which the settings files
/// do not take. Interfaces that are down are left out where the system says
/// so.
pub fn interfaces() -> io::Result<Vec<Interface>> {
    let mut out: Vec<Interface> = if_addrs::get_if_addrs()?
        .into_iter()
        .filter(|i| i.is_oper_up() || i.is_loopback())
        .filter(|i| !(i.ip().is_ipv6() && i.is_link_local()))
        .map(|i| Interface {
            loopback: i.is_loopback(),
            addr: i.ip(),
            name: i.name,
        })
        .collect();
    out.sort_by(|a, b| {
        (a.addr.is_ipv6(), a.loopback, a.addr).cmp(&(b.addr.is_ipv6(), b.loopback, b.addr))
    });
    out.dedup_by(|a, b| a.addr == b.addr);
    Ok(out)
}

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
    fn the_address_list_has_loopback_last_and_no_link_local() {
        // Every CI machine has a loopback interface.
        let list = interfaces().expect("interface table");
        assert!(
            list.iter().any(|i| i.loopback && i.addr.is_ipv4()),
            "{list:?}"
        );
        for pair in list.windows(2) {
            let key = |i: &Interface| (i.addr.is_ipv6(), i.loopback, i.addr);
            assert!(
                key(&pair[0]) < key(&pair[1]),
                "sorted, no repeats: {list:?}"
            );
        }
        assert!(!list.iter().any(|i| match i.addr {
            IpAddr::V6(a) => (a.segments()[0] & 0xffc0) == 0xfe80,
            IpAddr::V4(_) => false,
        }));
        let lo = list.iter().find(|i| i.loopback).unwrap();
        assert!(lo.label().ends_with("this machine only)"), "{}", lo.label());
    }

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
