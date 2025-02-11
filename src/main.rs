use std::time::Duration;

use backend::{
    cameras::{CameraBackend, DefaultCameraBackend},
    servers::{DefaultServerBackend, ServerBackend},
};
use frontend::{
    main_app::{MainApp, MainAppMessage},
    setup::{Setup, SetupMessage},
};
use iced::{keyboard::Key, theme::Palette, Font, Task};

mod backend;
mod frontend;

enum AppPage<
    C: crate::backend::cameras::CameraBackend + 'static,
    S: crate::backend::servers::ServerBackend + 'static,
> {
    Setup(Setup<C, S>),
    MainApp(MainApp<C, S>),
}

struct PhotoBoothApplication<
    C: crate::backend::cameras::CameraBackend + 'static,
    S: crate::backend::servers::ServerBackend + 'static,
> {
    page: AppPage<C, S>,
    server_backend: S,
}

#[derive(Debug, Clone)]
enum PhotoBoothMessage<
    C: crate::backend::cameras::CameraBackend + 'static,
    S: crate::backend::servers::ServerBackend + 'static,
> {
    Setup(SetupMessage<C>),
    MainApp(MainAppMessage<S>),
    Tick,
    SpaceReleased,
    EscapeReleased,
    UpReleased,
    DownReleased,
}

#[derive(Debug, Clone, Copy)]
enum KeyMessage {
    Space,
    Up,
    Down,
}

impl<
        C: crate::backend::cameras::CameraBackend + 'static + Clone,
        S: crate::backend::servers::ServerBackend + 'static,
    > PhotoBoothApplication<C, S>
{
    fn update(&mut self, message: PhotoBoothMessage<C, S>) -> Task<PhotoBoothMessage<C, S>> {
        match message {
            PhotoBoothMessage::Setup(msg) => match &mut self.page {
                AppPage::Setup(page) => {
                    let update_task = page.update(msg).map(PhotoBoothMessage::Setup);
                    if let Some(new_page) = page.new_page.take() {
                        let (new_page, new_task) = *new_page;
                        self.page = new_page;
                        update_task.chain(new_task)
                    } else {
                        update_task
                    }
                }
                _ => Task::none(),
            },
            PhotoBoothMessage::MainApp(msg) => match &mut self.page {
                AppPage::MainApp(page) => {
                    let update_task = page
                        .update(msg, self.server_backend.clone())
                        .map(PhotoBoothMessage::MainApp);
                    if let Some(new_page) = page.new_page.take() {
                        let (new_page, new_task) = *new_page;
                        self.page = new_page;
                        update_task.chain(new_task)
                    } else {
                        update_task
                    }
                }
                _ => Task::none(),
            },
            PhotoBoothMessage::Tick => match &mut self.page {
                AppPage::MainApp(page) => page
                    .update(MainAppMessage::Tick, self.server_backend.clone())
                    .map(PhotoBoothMessage::MainApp),
                _ => Task::none(),
            },
            PhotoBoothMessage::SpaceReleased
            | PhotoBoothMessage::DownReleased
            | PhotoBoothMessage::UpReleased => match &mut self.page {
                AppPage::MainApp(page) => page
                    .update(
                        MainAppMessage::KeyReleased(match message {
                            PhotoBoothMessage::SpaceReleased => KeyMessage::Space,
                            PhotoBoothMessage::DownReleased => KeyMessage::Down,
                            PhotoBoothMessage::UpReleased => KeyMessage::Up,
                            _ => unreachable!(),
                        }),
                        self.server_backend.clone(),
                    )
                    .map(PhotoBoothMessage::MainApp),
                _ => Task::none(),
            },
            PhotoBoothMessage::EscapeReleased => iced::window::get_latest().then(|id| {
                iced::Task::batch([
                    iced::window::get_mode(id.unwrap()).then(move |mode| {
                        iced::window::change_mode(
                            id.unwrap(),
                            match mode {
                                iced::window::Mode::Fullscreen => iced::window::Mode::Windowed,
                                iced::window::Mode::Windowed => iced::window::Mode::Fullscreen,
                                iced::window::Mode::Hidden => iced::window::Mode::Windowed,
                            },
                        )
                    }),
                    iced::window::toggle_decorations(id.unwrap()),
                ])
            }),
        }
    }

    fn view(&self) -> iced::Element<PhotoBoothMessage<C, S>> {
        match &self.page {
            AppPage::MainApp(page) => page
                .view(&self.server_backend)
                .map(PhotoBoothMessage::MainApp),
            AppPage::Setup(page) => page.view().map(PhotoBoothMessage::Setup),
        }
    }

    fn subscription(&self) -> iced::Subscription<PhotoBoothMessage<C, S>> {
        const FPS: f32 = 30.0;
        iced::Subscription::batch([
            iced::time::every(Duration::from_secs_f32(1.0 / FPS))
                .map(|_tick| PhotoBoothMessage::Tick),
            iced::keyboard::on_key_press(|key, _modifiers| match key {
                Key::Named(iced::keyboard::key::Named::Space)
                | Key::Named(iced::keyboard::key::Named::Enter) => {
                    Some(PhotoBoothMessage::SpaceReleased)
                }
                Key::Named(iced::keyboard::key::Named::Escape) => {
                    Some(PhotoBoothMessage::EscapeReleased)
                }
                Key::Named(iced::keyboard::key::Named::PageUp)
                | Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                    Some(PhotoBoothMessage::UpReleased)
                }
                Key::Named(iced::keyboard::key::Named::PageDown)
                | Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                    Some(PhotoBoothMessage::DownReleased)
                }
                _ => None,
            }),
        ])
    }
}

fn main() -> iced::Result {
    type CameraBackend = DefaultCameraBackend;
    type ServerBackend = DefaultServerBackend;

    CameraBackend::initialize().expect("failed to initialize camera backend");

    iced::application(
        "Shrimp Games Check-in",
        PhotoBoothApplication::update,
        PhotoBoothApplication::view,
    )
    .font(include_bytes!(
        "../assets/fonts/Noto_Color_Emoji/NotoColorEmoji-Regular.ttf"
    ))
    .font(include_bytes!(
        "../assets/fonts/Poor_Story/PoorStory-Regular.ttf"
    ))
    .default_font(Font::with_name("Poor Story"))
    .theme(|_| {
        iced::Theme::custom(
            "Shrimp Game".to_owned(),
            Palette {
                background: iced::Color::from_rgb8(0x4e, 0x2a, 0x25),
                text: iced::Color::from_rgb8(0xff, 0xff, 0xff),
                primary: iced::Color::from_rgb8(0xf8, 0x46, 0xaa),
                success: iced::Color::from_rgb8(0x00, 0xff, 0x00),
                danger: iced::Color::from_rgb8(0xff, 0x00, 0x00),
            },
        )
    })
    .subscription(PhotoBoothApplication::subscription)
    .run_with(|| {
        let server_backend = ServerBackend::new().expect("failed to initialize server backend");
        (
            PhotoBoothApplication::<CameraBackend, ServerBackend> {
                page: AppPage::Setup(Setup::new()),
                server_backend,
            },
            Task::none(),
        )
    })
}
