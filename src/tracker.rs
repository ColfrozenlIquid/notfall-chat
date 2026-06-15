use std::{
    collections::HashMap,
    ffi::CStr,
    time::{Duration, Instant},
};

use crate::ffi;

pub struct PeerTracker {
    peers: HashMap<PeerKey, DiscoveredPeer>,
    ttl: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerKey {
    pub ip: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub ip: String,
    pub name: String,
    pub port: u16,
    pub first_seen: Instant,
    pub last_seen: Instant,
}

impl PeerTracker {
    pub fn new(ttl: Duration) -> Self {
        Self {
            peers: HashMap::new(),
            ttl,
        }
    }

    pub fn poll(&mut self) {
        loop {
            let mut raw: ffi::C_DiscoveredPeer = unsafe { std::mem::zeroed() };

            let result = unsafe { ffi::discovery_listener_pop(std::ptr::addr_of_mut!(raw)) };

            if result != 0 {
                break;
            }

            let ip = unsafe {
                CStr::from_ptr(raw.sender_ip.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            };

            let name = raw
                .name
                .iter()
                .take_while(|&&b| b != 0)
                .copied()
                .collect::<Vec<u8>>();

            let name = String::from_utf8_lossy(&name).into_owned();

            let key = PeerKey {
                ip: ip.clone(),
                name: name.clone(),
            };
            let now = Instant::now();

            self.peers
                .entry(key)
                .and_modify(|p| p.last_seen = now)
                .or_insert_with(|| {
                    println!("New peer: {name} @ {ip}:{}", raw.port);
                    DiscoveredPeer {
                        ip,
                        name,
                        port: raw.port,
                        first_seen: now,
                        last_seen: now,
                    }
                });
        }
    }

    pub fn evict_stale(&mut self) {
        let now = Instant::now();
        self.peers.retain(|_, peer| {
            let keep = now.duration_since(peer.last_seen) < self.ttl;
            if !keep {
                println!("Peer expired: {} @ {}", peer.name, peer.ip);
            }
            keep
        });
    }

    pub fn peers(&self) -> impl Iterator<Item = &DiscoveredPeer> {
        self.peers.values()
    }
}
