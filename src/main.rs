use std::time::Duration;

use backend::{
    cameras::{CameraBackend, DefaultCameraBackend},
    servers::{DefaultServerBackend, ServerBackend},
};
use frontend::{
    main_app::{MainApp, MainAppMessage},
    setup::{Setup, SetupMessage},
};
use iced::{keyboard::Key, Font, Task};

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
            PhotoBoothMessage::SpaceReleased => match &mut self.page {
                AppPage::MainApp(page) => page
                    .update(MainAppMessage::SpaceReleased, self.server_backend.clone())
                    .map(PhotoBoothMessage::MainApp),
                _ => Task::none(),
            },
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
            iced::keyboard::on_key_release(|key, _modifiers| match key {
                Key::Named(iced::keyboard::key::Named::Space) => {
                    Some(PhotoBoothMessage::SpaceReleased)
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
        "Photo Booth v2",
        PhotoBoothApplication::update,
        PhotoBoothApplication::view,
    )
    .subscription(PhotoBoothApplication::subscription)
    .font(include_bytes!("../assets/NotoSansJP-Regular.ttf"))
    .default_font(Font::with_name("Noto Sans JP"))
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
