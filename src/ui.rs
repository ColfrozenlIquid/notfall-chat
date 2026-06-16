use std::{
    collections::HashMap,
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
    incoming_connections: std::sync::mpsc::Receiver<ConnectionHandle>,
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
    fn new(
        tracker: Arc<Mutex<PeerTracker>>,
        name: String,
        rx: std::sync::mpsc::Receiver<ConnectionHandle>,
    ) -> (Self, Task<Message>) {
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
            incoming_connections: rx,
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
                while let Ok(handle) = self.incoming_connections.try_recv() {
                    let peer_name = "laptop".to_string();
                    self.connections.insert(peer_name, handle);
                }
                let mut received = vec![];
                for (peer, handle) in &self.connections {
                    match handle.try_receive() {
                        Ok(Some(msg)) => received.push((peer.clone(), msg)),
                        Ok(None) => {}
                        Err(e) => eprintln!("recv error from {peer}: {e}"),
                    }
                }
                for (peer, msg) in received {
                    let message = UserMessage {
                        timestamp: Utc::now(),
                        content: msg,
                        user: peer.clone(),
                    };
                    self.messages.entry(peer).or_default().push(message);
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
                self.connections.insert(peer_name.clone(), handle);
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
            Message::MessageReceived(peer, content) => {
                self.messages
                    .entry(peer.clone())
                    .or_default()
                    .push(UserMessage {
                        timestamp: Utc::now(),
                        content,
                        user: peer,
                    });
            }
        }
        Task::none()
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::time::every(Duration::from_millis(100)).map(|_| Message::Tick)
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

pub fn run_ui(
    tracker: Arc<Mutex<PeerTracker>>,
    name: String,
    rx: std::sync::mpsc::Receiver<ConnectionHandle>,
) -> iced::Result {
    let rx = std::sync::Mutex::new(Some(rx));

    let mut app = iced::application(
        move || {
            let rx = rx.lock().unwrap().take().expect("run_ui called twice");
            App::new(Arc::clone(&tracker), name.clone(), rx)
        },
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
