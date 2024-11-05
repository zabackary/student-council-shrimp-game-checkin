use iced::{
    widget::{button, column, container, pick_list, text},
    Alignment, Element, Length, Task,
};

use crate::{AppPage, MainAppMessage, PhotoBoothMessage};

use super::{camera_feed::CameraFeed, main_app::MainApp};

#[derive(Debug, Clone)]
pub enum SetupMessage<C: crate::backend::cameras::CameraBackend + 'static> {
    CameraSelected(C::EnumeratedCamera),
    StartPressed,
}

pub struct Setup<
    C: crate::backend::cameras::CameraBackend + 'static,
    S: crate::backend::servers::ServerBackend + 'static,
> {
    camera_options: Vec<C::EnumeratedCamera>,
    camera_option: Option<C::EnumeratedCamera>,
    pub new_page: Option<Box<(AppPage<C, S>, Task<PhotoBoothMessage<C, S>>)>>,
}

impl<
        C: crate::backend::cameras::CameraBackend + 'static,
        S: crate::backend::servers::ServerBackend + 'static,
    > Setup<C, S>
{
    pub fn new() -> Self {
        Self {
            camera_options: C::enumerate_cameras().unwrap(),
            camera_option: None,
            new_page: None,
        }
    }

    pub fn update(&mut self, message: SetupMessage<C>) -> Task<SetupMessage<C>> {
        match message {
            SetupMessage::CameraSelected(new) => {
                self.camera_option = Some(new);
                Task::none()
            }
            SetupMessage::StartPressed => {
                let (feed, task) = CameraFeed::new(
                    C::open_camera(self.camera_option.clone().unwrap()).unwrap(),
                    Default::default(),
                );
                self.new_page = Some(Box::new((
                    AppPage::MainApp(MainApp::new(feed)),
                    task.map(MainAppMessage::Camera)
                        .map(PhotoBoothMessage::MainApp),
                )));
                iced::window::get_latest().then(|id| {
                    iced::Task::batch([
                        iced::window::change_mode(id.unwrap(), iced::window::Mode::Fullscreen),
                        iced::window::toggle_decorations(id.unwrap()),
                    ])
                })
            }
        }
    }

    pub fn view(&self) -> Element<SetupMessage<C>> {
        container(
            container(
                column([
                    text("Setup").size(32).into(),
                    pick_list(
                        self.camera_options.as_ref(),
                        self.camera_option.as_ref(),
                        SetupMessage::CameraSelected,
                    )
                    .into(),
                    button("Start")
                        .on_press_maybe(
                            self.camera_option
                                .is_some()
                                .then_some(SetupMessage::StartPressed),
                        )
                        .into(),
                ])
                .align_x(Alignment::Center)
                .spacing(8),
            )
            .padding(8)
            .style(container::rounded_box),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}
