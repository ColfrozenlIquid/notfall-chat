use iced::{
    Element, Font, Size, Task, Theme,
    widget::{column, container, text},
    window::Settings,
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

fn main() -> iced::Result {
    let mut app = iced::application(App::new, App::update, App::view)
        .window(Settings {
            size: Size::new(WINDOW_WIDTH, WINDOW_HEIGHT),
            decorations: true,
            position: iced::window::Position::Centered,
            resizable: true,
            blur: true,
            transparent: true,
            ..Default::default()
        })
        .theme(App::theme)
        .default_font(Font::MONOSPACE);

    for (_, bytes) in FONTS {
        app = app.font(bytes.iter().as_slice());
    }

    let _ = app.default_font(Font::with_name("JetBrains Mono")).run();

    Ok(())
}

pub struct App {}

pub enum Message {}

impl App {
    fn new() -> (Self, Task<Message>) {
        let app = App {};
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
