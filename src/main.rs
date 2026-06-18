pub mod ffi;
pub mod tracker;
pub mod ui;

use std::{
    env,
    ffi::CStr,
    io::Error,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{ffi::ConnectionHandle, tracker::PeerTracker, ui::run_ui};

#[derive(Debug)]
pub struct NetworkDevice {
    pub name: String,
    pub addr: String,
    pub is_ipv6: bool,
}

fn broadcast_discover(name: String, tcp_port: u16) {
    let mut buf = [0u8; ffi::DISCOVERY_NAME_LEN];
    let bytes = name.as_bytes();
    let copy_len = bytes.len().min(ffi::DISCOVERY_NAME_LEN - 1);
    buf[..copy_len].copy_from_slice(&bytes[..copy_len]);

    let result = unsafe { ffi::broadcast_discovery(buf.as_mut_ptr(), copy_len, tcp_port) };

    if result < 0 {
        let err = Error::last_os_error();
        eprintln!("setsockopt failed: {}", err);
    }
}

fn get_network_devices() -> Vec<NetworkDevice> {
    const MAX_DEVICES: usize = 16;
    let mut buf: [ffi::C_NetworkDevice; MAX_DEVICES] = unsafe { std::mem::zeroed() };
    let count = unsafe { ffi::get_network_devices(buf.as_mut_ptr(), MAX_DEVICES) };

    let mut result = Vec::with_capacity(count);
    for dev in &buf[..count as usize] {
        let name = unsafe { CStr::from_ptr(dev.name.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        let addr = unsafe { CStr::from_ptr(dev.addr.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        result.push(NetworkDevice {
            name,
            addr,
            is_ipv6: dev.is_ipv6 != 0,
        });
    }
    result
}

fn generate_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let adjectives = [
        "ancient",
        "blazing",
        "cosmic",
        "drifting",
        "electric",
        "frozen",
        "glowing",
        "hidden",
        "infinite",
        "jolting",
        "kinetic",
        "lunar",
        "mystic",
        "neon",
        "orbital",
        "phantom",
        "quantum",
        "rogue",
        "stellar",
        "turbo",
        "umbral",
        "void",
        "wandering",
        "xenon",
        "zephyr",
    ];

    let nouns = [
        "albatross",
        "basilisk",
        "condor",
        "drifter",
        "eclipse",
        "falcon",
        "ghost",
        "harbinger",
        "ironclad",
        "jackal",
        "kraken",
        "lynx",
        "mongoose",
        "nebula",
        "osprey",
        "panther",
        "quasar",
        "raptor",
        "sphinx",
        "tempest",
        "ulysses",
        "vortex",
        "wraith",
        "xenolith",
        "yeti",
    ];

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as usize)
        .unwrap_or(42);

    let adj = adjectives[seed % adjectives.len()];
    let noun = nouns[(seed / adjectives.len() + seed) % nouns.len()];

    format!("{}-{}", adj, noun)
}

fn main() -> iced::Result {
    let args: Vec<String> = env::args().collect();
    let headless = args.contains(&"--headless".to_string());
    let name = args
        .windows(2)
        .find(|w| w[0] == "--name")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| generate_name());

    println!("Assigned self to name: {}", name);

    let devices = get_network_devices();
    println!("Network Devices: {:?}", devices);

    std::thread::spawn({
        let name = name.clone();
        move || {
            broadcast_discover(name, 50000);
        }
    });

    unsafe { ffi::discovery_listener_start() };

    let (tx, rx) = std::sync::mpsc::channel::<ConnectionHandle>();

    std::thread::spawn(move || {
        let tx_box = Box::new(tx);
        let userdata = Box::into_raw(tx_box) as *mut std::ffi::c_void;
        unsafe { ffi::run_server(12345, ffi::on_accept_callback, userdata) };
    });

    let tracker = Arc::new(Mutex::new(PeerTracker::new(Duration::from_secs(30))));
    let tracker_bg = Arc::clone(&tracker);

    std::thread::spawn(move || {
        loop {
            {
                let mut t = tracker_bg.lock().unwrap();
                t.poll();
                t.evict_stale();
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    });

    if headless {
        println!("Running in headless mode as '{name}'. Press Ctrl+C to exit.");
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    let tracker_clone = Arc::clone(&tracker);

    let _ = run_ui(tracker_clone, name, rx);

    Ok(())
}
