use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, Utc};
use iced::{
    Color, Element, Font, Length, Size, Task, Theme,
    widget::{self, button, column, container, row, text, text_input},
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
    name: String,
    addr: String,
    port: u16,
    tracker: std::sync::Arc<Mutex<PeerTracker>>,
    peers: Vec<DiscoveredPeer>,
    connections: HashMap<String, ConnectionHandle>,
    messages: HashMap<String, Vec<UserMessage>>,
    selected_peer: Option<String>,
    content: String,
}

#[derive(Clone)]
pub enum Message {
    PeerClicked(String),
    Tick,
    PeerConnect(String, String),
    Connected(ConnectionHandle, String),
    ConnectFailed(String),
    ContentChanged(String),
    SendMessage,
    MessageReceived(String, String),
}

pub struct MessageHistory {
    messages: Vec<UserMessage>,
}

#[derive(Debug, Clone)]
pub struct UserMessage {
    timestamp: DateTime<Utc>,
    content: String,
    user: String,
}

pub enum User {
    SELF,
    PARTNER,
}

impl App {
    fn new(tracker: Arc<Mutex<PeerTracker>>, name: String) -> (Self, Task<Message>) {
        let app = App {
            name,
            addr: String::new(),
            port: 0,
            tracker,
            peers: vec![],
            connections: HashMap::new(),
            messages: HashMap::new(),
            selected_peer: None,
            content: String::new(),
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
            Message::PeerConnect(peer_name, peer_ip) => {
                self.selected_peer = Some(peer_name.clone());

                return Task::perform(
                    async move { ConnectionHandle::connect(&peer_ip, 12345) },
                    |result| match result {
                        Ok(handle) => Message::Connected(handle, peer_name),
                        Err(e) => Message::ConnectFailed(e),
                    },
                );
            }
            Message::Connected(handle, peer_name) => {
                let handle_clone = handle.clone();
                self.connections.insert(peer_name.clone(), handle);

                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || handle_clone.receive())
                            .await
                            .map_err(|e| e.to_string())
                            .and_then(|r| r)
                    },
                    |result| match result {
                        Ok(msg) => Message::MessageReceived(peer_name, msg),
                        Err(e) => Message::ConnectFailed(e),
                    },
                );
            }
            Message::ConnectFailed(e) => {
                eprintln!("connection failed: {e}");
            }
            Message::ContentChanged(content) => {
                self.content = content;
            }
            Message::SendMessage => {
                if self.content.is_empty() {
                    return Task::none();
                }

                let Some(peer) = self.selected_peer.clone() else {
                    return Task::none();
                };

                let msg = UserMessage {
                    timestamp: Utc::now(),
                    content: self.content.clone(),
                    user: self.name.clone(),
                };

                self.messages
                    .entry(peer.clone())
                    .or_default()
                    .push(msg.clone());

                self.content.clear();

                if let Some(conn) = self.connections.get(&peer) {
                    let conn = conn.clone();
                    return Task::perform(
                        async move { ConnectionHandle::send(&conn, &msg.content.to_string()) },
                        |result| match result {
                            Ok(_) => Message::Tick,
                            Err(e) => Message::ConnectFailed(e),
                        },
                    );
                }
            }
            Message::MessageReceived(peer_name, content) => {
                let msg = UserMessage {
                    timestamp: Utc::now(),
                    content,
                    user: peer_name.clone(),
                };

                self.messages
                    .entry(peer_name.clone())
                    .or_default()
                    .push(msg);

                if let Some(handle) = self.connections.get(&peer_name) {
                    let handle = handle.clone();
                    return Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || handle.receive())
                                .await
                                .map_err(|e| e.to_string())
                                .and_then(|r| r)
                        },
                        move |result| match result {
                            Ok(msg) => Message::MessageReceived(peer_name.clone(), msg),
                            Err(e) => Message::ConnectFailed(e),
                        },
                    );
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
            col.push(
                button(text(format!("{} @ {}", peer.name, peer.ip)).size(TEXT_SIZE))
                    .width(Length::Fill)
                    .on_press(Message::PeerConnect(peer.name.clone(), peer.ip.clone()))
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
        //
        let message_entry = text_input("Type something here", &self.content)
            .on_input(Message::ContentChanged)
            .on_submit(Message::SendMessage)
            .size(TEXT_SIZE)
            .style(|theme, status| {
                let mut style = text_input::default(theme, status);
                style.border.width = 0.0;
                style.background = iced::Background::Color(Color::TRANSPARENT);
                style
            });

        let layout = row![
            column![peer_list].width(Length::Fill),
            column![text("Chat").size(TEXT_SIZE), chat, message_entry].width(Length::Fill)
        ]
        .padding(PADDING);

        container(layout).width(Length::Fill).into()
    }

    fn theme(_: &App) -> Theme {
        Theme::KanagawaDragon
    }
}

pub fn run_ui(tracker: Arc<Mutex<PeerTracker>>, name: String) -> iced::Result {
    let mut app = iced::application(
        move || App::new(Arc::clone(&tracker), name.clone()),
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
