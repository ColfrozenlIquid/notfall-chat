use chrono::{DateTime, Utc};
use iced::{
    Element, Font, Size, Task, Theme,
    widget::{column, container, text},
    window::Settings,
};
use std::{ffi::CStr, io::Error};

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
    pub const NAME_LEN: usize = 32;

    #[repr(C)]
    pub struct C_NetworkDevice {
        pub name: [c_char; DEVICE_NAME_LEN],
        pub addr: [c_char; DEVICE_ADDR_LEN],
        pub is_ipv6: i32,
    }

    unsafe extern "C" {
        pub fn get_network_devices(devices: *mut C_NetworkDevice, len: usize) -> usize;
        pub fn broadcast_discovery(name: *mut u8, name_len: usize, tcp_port: u16) -> i32;
    }
}

#[derive(Debug)]
pub struct NetworkDevice {
    pub name: String,
    pub addr: String,
    pub is_ipv6: bool,
}

fn broadcast_discover(name: String, tcp_port: u16) {
    let mut buf = [0u8; ffi::NAME_LEN];
    let bytes = name.as_bytes();
    let copy_len = bytes.len().min(ffi::NAME_LEN - 1);
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

    if count < 0 {
        eprintln!("get_network_devices failed");
        return Vec::new();
    }

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

    broadcast_discover("daniel".to_string(), 50000);

    // let mut app = iced::application(App::new, App::update, App::view)
    //     .window(Settings {
    //         size: Size::new(WINDOW_WIDTH, WINDOW_HEIGHT),
    //         decorations: true,
    //         position: iced::window::Position::Centered,
    //         resizable: true,
    //         blur: true,
    //         transparent: true,
    //         ..Default::default()
    //     })
    //     .theme(App::theme)
    //     .default_font(Font::MONOSPACE);

    // for (_, bytes) in FONTS {
    //     app = app.font(bytes.iter().as_slice());
    // }

    // let _ = app.default_font(Font::with_name("JetBrains Mono")).run();

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
}

pub enum Message {}

impl App {
    fn new() -> (Self, Task<Message>) {
        let app = App {
            addr: todo!(),
            port: todo!(),
        };
        (app, Task::none())
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let header = column![text!("Hello World").size(TEXT_SIZE)].padding(PADDING);
        container(header).into()
    }

    fn theme(_: &App) -> Theme {
        Theme::KanagawaDragon
    }
}
