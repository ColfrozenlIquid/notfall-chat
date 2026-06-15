use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, Utc};
use iced::{
    Element, Font, Length, Size, Task, Theme,
    widget::{self, button, column, container, row, text},
    window::Settings,
};

use crate::{
    ffi::ConnectionHandle,
    tracker::{DiscoveredPeer, PeerTracker},
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

pub struct App {
    addr: String,
    port: u16,
    tracker: std::sync::Arc<Mutex<PeerTracker>>,
    peers: Vec<DiscoveredPeer>,
    connections: HashMap<String, ConnectionHandle>,
    messages: HashMap<String, Vec<UserMessage>>,
    selected_peer: Option<String>,
}

#[derive(Clone)]
pub enum Message {
    PeerClicked(String),
    Tick,
    PeerConnect(String),
    Connected(ConnectionHandle, String),
    ConnectFailed(String),
}

pub struct MessageHistory {
    messages: Vec<UserMessage>,
}

pub struct UserMessage {
    id: String,
    timestamp: DateTime<Utc>,
    content: String,
    user: String,
}

pub enum User {
    SELF,
    PARTNER,
}

impl App {
    fn new(tracker: Arc<Mutex<PeerTracker>>) -> (Self, Task<Message>) {
        let app = App {
            addr: String::new(),
            port: 0,
            tracker,
            peers: vec![],
            connections: HashMap::new(),
            messages: HashMap::new(),
            selected_peer: None,
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
            Message::PeerConnect(peer_ip) => {
                let peer_ip_clone = peer_ip.clone();
                return Task::perform(
                    async move { ConnectionHandle::connect(&peer_ip_clone, 12345) },
                    |result| match result {
                        Ok(handle) => Message::Connected(handle, peer_ip),
                        Err(e) => Message::ConnectFailed(e),
                    },
                );
            }
            Message::Connected(handle, peer_ip) => {
                self.connections.insert(peer_ip, handle);
            }
            Message::ConnectFailed(e) => {
                eprintln!("connection failed: {e}");
            }
        }
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_millis(500)).map(|_| Message::Tick)
    }

    fn view(&self) -> Element<'_, Message> {
        let peer_list = self.peers.iter().fold(column![].spacing(8), |col, peer| {
            col.push(
                button(text(format!("{} @ {}", peer.name, peer.ip)).size(TEXT_SIZE))
                    .width(Length::Fill)
                    .on_press(Message::PeerConnect(peer.ip.clone()))
                    .style(widget::button::text),
            )
        });

        let chat = self
            .selected_peer
            .as_deref()
            .and_then(|peer| self.messages.get(peer))
            .map(|msgs| msgs.as_slice())
            .unwrap_or(&[])
            .iter()
            .fold(column![].spacing(8), |col, msg| {
                col.push(
                    text(format!("{} @ {}: {}", msg.user, msg.timestamp, msg.content))
                        .size(TEXT_SIZE)
                        .width(Length::Fill),
                )
            });

        // let chat = self.text(format!("{} @ {}: {}"), )

        let layout = row![
            column![peer_list].width(Length::Fill),
            column![text("Chat").size(TEXT_SIZE), chat].width(Length::Fill)
        ]
        .padding(PADDING);

        container(layout).width(Length::Fill).into()
    }

    fn theme(_: &App) -> Theme {
        Theme::KanagawaDragon
    }
}

pub fn run_ui(tracker: Arc<Mutex<PeerTracker>>) -> iced::Result {
    let mut app = iced::application(
        move || App::new(Arc::clone(&tracker)),
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
