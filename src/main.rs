use chrono::{DateTime, Utc};
use iced::{
    Element, Font, Length, Size, Task, Theme,
    widget::{button, column, container, row, text},
    window::Settings,
};
use std::{
    collections::HashMap,
    ffi::CStr,
    io::Error,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const WINDOW_WIDTH: f32 = 1400.0;
const WINDOW_HEIGHT: f32 = 1400.0;

const TEXT_SIZE: f32 = 28.0;
const PADDING: f32 = 28.0;

const FONTS: &[(&str, &[u8])] = &[
    (
        "IBMPlexMono",
        include_bytes!("../fonts/IBMPlexMono-Regular.ttf"),
    ),
    (
        "JetBrainsMono",
        include_bytes!("../fonts/JetBrainsMono-Regular.ttf"),
    ),
];

mod ffi {
    use std::os::raw::c_char;

    pub const DEVICE_NAME_LEN: usize = 16;
    pub const DEVICE_ADDR_LEN: usize = 46;
    pub const DISCOVERY_NAME_LEN: usize = 32;
    pub const INET_ADDRSTRLEN: usize = 16;

    #[repr(C)]
    pub struct C_NetworkDevice {
        pub name: [c_char; DEVICE_NAME_LEN],
        pub addr: [c_char; DEVICE_ADDR_LEN],
        pub is_ipv6: i32,
    }

    #[repr(C)]
    pub struct C_DiscoveredPeer {
        pub timestamp: u64,
        pub port: u16,
        pub sender_ip: [c_char; INET_ADDRSTRLEN],
        pub name: [u8; DISCOVERY_NAME_LEN],
    }

    unsafe extern "C" {
        pub fn get_network_devices(devices: *mut C_NetworkDevice, len: usize) -> usize;
        pub fn broadcast_discovery(name: *mut u8, name_len: usize, tcp_port: u16) -> i32;
        pub fn discovery_listener_start();
        pub fn discovery_listener_pop(out: *mut C_DiscoveredPeer) -> i32;
    }
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

pub struct PeerTracker {
    peers: HashMap<PeerKey, DiscoveredPeer>,
    ttl: Duration,
}

impl PeerTracker {
    fn new(ttl: Duration) -> Self {
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

fn run_discovery(tracker: &mut PeerTracker) {
    loop {
        tracker.poll();
        tracker.evict_stale();
        std::thread::sleep(Duration::from_millis(500));
    }
}

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
        let err = Error::last_os_error(); // reads errno before anything else runs
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
    let devices = get_network_devices();
    println!("Network Devices: {:?}", devices);

    // spawn broadcaster — it loops forever so it needs its own thread
    std::thread::spawn(|| {
        broadcast_discover("pop-os".to_string(), 50000);
    });

    unsafe { ffi::discovery_listener_start() };

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

    let tracker_clone = Arc::clone(&tracker);

    let mut app = iced::application(
        move || App::new(Arc::clone(&tracker_clone)),
        App::update,
        App::view,
    )
    .window(Settings {
        size: Size::new(WINDOW_WIDTH, WINDOW_HEIGHT),
        decorations: true,
        position: iced::window::Position::Centered,
        resizable: true,
        blur: true,
        transparent: true,
        ..Default::default()
    })
    .subscription(App::subscription)
    .theme(App::theme)
    .default_font(Font::MONOSPACE);

    for (_, bytes) in FONTS {
        app = app.font(bytes.iter().as_slice());
    }

    let _ = app.default_font(Font::with_name("JetBrains Mono")).run();

    Ok(())
}

pub struct MessageHistory {
    messages: Vec<UserMessage>,
}

pub struct UserMessage {
    id: String,
    timestamp: DateTime<Utc>,
    content: String,
    user: User,
}

pub enum User {
    SELF,
    PARTNER,
}

pub struct App {
    addr: String,
    port: u16,
    tracker: Arc<Mutex<PeerTracker>>,
    peers: Vec<DiscoveredPeer>,
}

#[derive(Debug, Clone)]
pub enum Message {
    PeerClicked(String),
    Tick,
}

impl App {
    fn new(tracker: Arc<Mutex<PeerTracker>>) -> (Self, Task<Message>) {
        let app = App {
            addr: String::new(),
            port: 0,
            tracker,
            peers: vec![],
        };
        (app, Task::none())
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::PeerClicked(peer) => println!("pressed on {:?}", peer),
            Message::Tick => {
                if let Ok(t) = self.tracker.lock() {
                    self.peers = t.peers().cloned().collect();
                }
            }
        }
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_millis(500)).map(|_| Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let peer_list = self.peers.iter().fold(column![].spacing(8), |col, peer| {
            col.push(text(format!("{} @ {}", peer.name, peer.ip)).size(TEXT_SIZE))
                .width(Length::Fill)
        });

        let layout = row![
            column![peer_list].width(Length::Fill),
            column![text!("Column 2").size(TEXT_SIZE)].width(Length::Fill)
        ]
        .padding(PADDING);

        container(layout).width(Length::Fill).into()
    }

    fn theme(_: &App) -> Theme {
        Theme::KanagawaDragon
    }
}
