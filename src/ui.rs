use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, Utc};
use iced::{
    Color, Element, Font, Length, Size, Task, Theme,
    widget::{self, button, column, container, row, rule, scrollable, text, text_input},
    window::Settings,
};

use crate::{
    ffi::ConnectionHandle,
    tracker::{DiscoveredPeer, PeerTracker},
};

const WINDOW_WIDTH: f32 = 1400.0;
const WINDOW_HEIGHT: f32 = 1000.0;

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
    connections: HashMap<String, (ConnectionHandle, Connection)>,
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
    StatsUpdated(String, f64, f64, f64),
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

#[derive(Clone)]
enum ConnectionStatus {
    CONNECTED,
    DISCONNECTED,
}

#[derive(Clone)]
pub struct Connection {
    user: String,
    status: ConnectionStatus,
    srtt: f64,
    rttvar: f64,
    loss_rate: f64,
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
                    let handle_clone = handle.clone();
                    self.connections.insert(
                        peer_name.clone(),
                        (
                            handle,
                            Connection {
                                user: peer_name.clone(),
                                status: ConnectionStatus::CONNECTED,
                                srtt: 0.0,
                                rttvar: 0.0,
                                loss_rate: 0.0,
                            },
                        ),
                    );

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
                self.connections.insert(
                    peer_name.clone(),
                    (
                        handle,
                        Connection {
                            user: peer_name.clone(),
                            status: ConnectionStatus::CONNECTED,
                            srtt: 0.0,
                            rttvar: 0.0,
                            loss_rate: 0.0,
                        },
                    ),
                );

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
            Message::StatsUpdated(peer, srtt, rttvar, loss_rate) => {
                if let Some((_, conn)) = self.connections.get_mut(&peer) {
                    conn.srtt = srtt;
                    conn.rttvar = rttvar;
                    conn.loss_rate = loss_rate;
                }
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

                if let Some((handle, _)) = self.connections.get(&peer) {
                    let handle = handle.clone();
                    let content = msg.content.clone();
                    let peer_for_result = peer.clone();

                    return Task::perform(
                        async move {
                            let result = ConnectionHandle::send(&handle, &content);
                            (handle, result)
                        },
                        move |(handle, result)| match result {
                            Ok(_) => Message::StatsUpdated(
                                peer_for_result.clone(),
                                handle.get_srtt(),
                                handle.get_rttvar(),
                                handle.get_loss_rate(),
                            ),
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
                            tokio::task::spawn_blocking(move || handle.0.receive())
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
        let peer_list = self
            .peers
            .iter()
            .enumerate()
            .fold(column![].spacing(8), |col, peer| {
                col.push(
                    button(
                        text(format!("{}. {} @ {}", peer.0, peer.1.name, peer.1.ip))
                            .size(TEXT_SIZE),
                    )
                    .width(Length::Fill)
                    .on_press(Message::PeerConnect(peer.1.name.clone(), peer.1.ip.clone()))
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
                col.push(row![
                    text(format!(
                        "{} {}: {}",
                        msg.timestamp.format("%H:%M:%S").to_string(),
                        msg.user,
                        msg.content
                    ))
                    .size(TEXT_SIZE)
                    .width(Length::Fill),
                ])
            });

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

        let vert_divider = rule::vertical(2);

        let chat_status = self
            .selected_peer
            .as_deref()
            .and_then(|peer| self.connections.get(peer))
            .map(|conn| &conn.1)
            .map(|conn| {
                text(format!(
                    "Status: Connected | SRTT: {:.2} ms | RTTVAR: {:.2} ms",
                    conn.srtt, conn.rttvar
                ))
                .size(20)
            });

        let layout = row![
            column![text!("====== PEERS ======").size(TEXT_SIZE), peer_list]
                .width(Length::FillPortion(1))
                .padding(10),
            vert_divider,
            column![
                text(format!(
                    "Chat with @ {}",
                    self.selected_peer.as_deref().unwrap_or("None")
                ))
                .size(TEXT_SIZE),
                chat_status,
                scrollable(chat).height(Length::Fill),
                message_entry
            ]
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .padding(10)
            .spacing(10)
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
