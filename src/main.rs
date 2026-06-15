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

use crate::{tracker::PeerTracker, ui::run_ui};

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

fn main() -> iced::Result {
    let args: Vec<String> = env::args().collect();
    let headless = args.contains(&"--headless".to_string());
    let name = args
        .windows(2)
        .find(|w| w[0] == "--name")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| "pop-os".to_string());

    let devices = get_network_devices();
    println!("Network Devices: {:?}", devices);

    std::thread::spawn({
        let name = name.clone();
        move || {
            broadcast_discover(name, 50000);
        }
    });

    unsafe { ffi::discovery_listener_start() };

    std::thread::spawn(move || {
        unsafe { ffi::run_server(12345) };
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
            std::thread::sleep(Duration::from_secs(60));
        }
    }

    let tracker_clone = Arc::clone(&tracker);

    let _ = run_ui(tracker_clone, name);

    Ok(())
}
