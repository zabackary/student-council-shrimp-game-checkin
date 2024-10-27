use std::time::Duration;

use backend::cameras::{nokhwa::NokhwaBackend, CameraBackend};
use frontend::{
    main_app::{MainApp, MainAppMessage},
    setup::{Setup, SetupMessage},
};
use iced::{keyboard::Key, Task};

mod backend;
mod frontend;

enum AppPage<C: crate::backend::cameras::CameraBackend + 'static> {
    Setup(Setup<C>),
    MainApp(MainApp<C>),
}

struct PhotoBoothApplication<C: crate::backend::cameras::CameraBackend + 'static> {
    page: AppPage<C>,
}

#[derive(Debug, Clone)]
enum PhotoBoothMessage<C: crate::backend::cameras::CameraBackend + 'static> {
    Setup(SetupMessage<C>),
    MainApp(MainAppMessage),
    Tick,
    SpaceReleased,
}

impl<C: crate::backend::cameras::CameraBackend + 'static + Clone> PhotoBoothApplication<C> {
    fn update(&mut self, message: PhotoBoothMessage<C>) -> Task<PhotoBoothMessage<C>> {
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
                    let update_task = page.update(msg).map(PhotoBoothMessage::MainApp);
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
                    .update(MainAppMessage::Tick)
                    .map(PhotoBoothMessage::MainApp),
                _ => Task::none(),
            },
            PhotoBoothMessage::SpaceReleased => match &mut self.page {
                AppPage::MainApp(page) => page
                    .update(MainAppMessage::SpaceReleased)
                    .map(PhotoBoothMessage::MainApp),
                _ => Task::none(),
            },
        }
    }

    fn view(&self) -> iced::Element<PhotoBoothMessage<C>> {
        match &self.page {
            AppPage::MainApp(page) => page.view().map(PhotoBoothMessage::MainApp),
            AppPage::Setup(page) => page.view().map(PhotoBoothMessage::Setup),
        }
    }

    fn subscription(&self) -> iced::Subscription<PhotoBoothMessage<C>> {
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
    type CameraBackend = NokhwaBackend;

    CameraBackend::initialize().expect("failed to initialize camera backend");

    iced::application(
        "Photo Booth v2",
        PhotoBoothApplication::update,
        PhotoBoothApplication::view,
    )
    .subscription(PhotoBoothApplication::subscription)
    .run_with(|| {
        (
            PhotoBoothApplication::<CameraBackend> {
                page: AppPage::Setup(Setup::new()),
            },
            Task::none(),
        )
    })
}
