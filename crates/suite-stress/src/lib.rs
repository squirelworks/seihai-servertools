//! Integration and stress rig for the Seihai server suite.
//!
//! [`lab`] writes a throwaway lab folder and starts `seihai-listserver`,
//! `seihai-gserver` (which starts `seihai-npcserver` with `npcserver =
//! auto`) on loopback. [`bot`] logs bot clients in with `seihai-core`'s own
//! codecs, one version profile each, and keeps them walking, chatting and
//! asking for files. [`rc`] runs `seihai-rc` against the suite now and then.
//! [`sample`] reads each server process's memory, CPU, threads and open
//! files once a second, and [`report`] writes what came out.
//!
//! Everything binds and connects to `127.0.0.1` only.

pub mod bot;
pub mod lab;
pub mod rc;
pub mod report;
pub mod sample;
pub mod stats;

/// The version labels a run uses when none are given: one or two per
/// protocol generation and era, from the oldest profile to 6.x.
pub const DEFAULT_VERSIONS: &[&str] = &[
    "1.21", "1.32", "1.39", "1.41", "2.05", "2.17", "2.22", "2.31", "3.04", "4.20r8", "5.12",
    "6.03r7",
];

/// The lab password every bot account uses. The lab gserver makes each
/// account on its first login, and the lab listserver runs in open mode.
pub const LAB_PASSWORD: &str = "labpw";

/// Refuse anything but a loopback address.
pub fn check_loopback(addr: &str) -> Result<(), String> {
    let host = addr.rsplit_once(':').map_or(addr, |(h, _)| h);
    if host == "127.0.0.1" || host == "localhost" {
        Ok(())
    } else {
        Err(format!(
            "{addr} is not loopback; the rig only runs on 127.0.0.1"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_loopback_passes() {
        assert!(check_loopback("127.0.0.1:14802").is_ok());
        assert!(check_loopback("localhost:1").is_ok());
        assert!(check_loopback("10.0.0.2:14802").is_err());
        assert!(check_loopback("0.0.0.0:14802").is_err());
    }

    #[test]
    fn default_versions_are_profiles() {
        for v in DEFAULT_VERSIONS {
            assert!(
                seihai_core::protocol::ClientVersion::from_label(v).is_some(),
                "{v}"
            );
        }
    }
}
