use std::time::Duration;

use anim::Animation;
use iced::{
    widget::{
        column, container,
        image::Handle,
        progress_bar, scrollable,
        scrollable::{AbsoluteOffset, Id},
        vertical_space, Space,
    },
    Alignment, ContentFit, Element, Length, Task,
};
use image::RgbaImage;

use crate::{AppPage, KeyMessage, PhotoBoothMessage};

use super::{
    camera_feed::{CameraFeed, CameraFeedOptions},
    loading_spinners,
    team_row::team_row,
    title_overlay::{supporting_text, title_overlay, title_text},
};

mod animations;

const PHOTO_ASPECT_RATIO: f32 = 3.0 / 2.0;
const PHOTO_COUNT: usize = 4;

enum CapturePhotosState {
    Countdown {
        current: usize,
        countdown_timeline: anim::Timeline<animations::countdown_circle::AnimationState>,
    },
    Capture {
        capture_timeline: anim::Timeline<animations::capture_flash::AnimationState>,
    },
    Preview {
        preview_timeline: anim::Timeline<animations::capture_preview::AnimationState>,
        captured_handle: Handle,
    },
}

enum MainAppState {
    PaymentRequired {
        show_error: bool,
    },
    Preview,
    CapturePhotosPrepare {
        ready_timeline: anim::Timeline<animations::ready::AnimationState>,
    },
    CapturePhotos {
        current: usize,
        state: CapturePhotosState,
    },
    Uploading {
        progress_timeline: anim::Timeline<f32>,
    },
    EditPrintUpsellBanner {
        progress_timeline: anim::Timeline<f32>,
        template_preview_timeline: anim::Timeline<animations::upsell_templates::AnimationState>,
        template_index: usize,
    },
}

#[derive(Debug, Clone)]
pub enum MainAppMessage<S: crate::backend::servers::ServerBackend + 'static> {
    Camera(super::camera_feed::CameraMessage),
    Tick,
    KeyReleased(KeyMessage),
    CaptureStill,
    Uploaded(Result<S::UploadHandle, String>),
}

pub struct MainApp<
    C: crate::backend::cameras::CameraBackend + 'static,
    S: crate::backend::servers::ServerBackend + 'static,
> {
    feed: CameraFeed<C::Camera>,
    state: MainAppState,
    captured_photos: Vec<RgbaImage>,
    previews: Vec<iced::widget::image::Handle>,
    pub new_page: Option<Box<(AppPage<C, S>, Task<PhotoBoothMessage<C, S>>)>>,
}

impl<
        C: crate::backend::cameras::CameraBackend + 'static,
        S: crate::backend::servers::ServerBackend + 'static,
    > MainApp<C, S>
{
    pub fn new(feed: CameraFeed<C::Camera>) -> (Self, Task<MainAppMessage<S>>) {
        (
            Self {
                feed,
                state: MainAppState::PaymentRequired { show_error: false },
                new_page: None,
                captured_photos: Vec::with_capacity(PHOTO_COUNT),
                previews: Vec::with_capacity(PHOTO_COUNT),
            },
            Task::none(),
        )
    }

    pub fn update(
        &mut self,
        message: MainAppMessage<S>,
        server_backend: S,
    ) -> Task<MainAppMessage<S>> {
        self.feed.update_options(
            if matches!(
                self.state,
                MainAppState::CapturePhotosPrepare { .. }
                    | MainAppState::CapturePhotos { .. }
                    | MainAppState::Preview
            ) {
                CameraFeedOptions {
                    blur: 1.0,
                    aspect_ratio: Some(PHOTO_ASPECT_RATIO),
                    mirror: true,
                    ..Default::default()
                }
            } else {
                CameraFeedOptions {
                    blur: 20.0, // 1/20th the resolution
                    aspect_ratio: None,
                    mirror: true,
                    ..Default::default()
                }
            },
        );

        match message {
            MainAppMessage::Camera(msg) => self.feed.update(msg).map(MainAppMessage::Camera),
            MainAppMessage::CaptureStill => {
                let image = self
                    .feed
                    .capture_still_sync(CameraFeedOptions {
                        aspect_ratio: Some(PHOTO_ASPECT_RATIO),
                        mirror: true,
                        ..Default::default()
                    })
                    .expect("failed to capture image");
                self.captured_photos.push(image);
                match &mut self.state {
                    MainAppState::CapturePhotos { state, .. } => {
                        *state = CapturePhotosState::Capture {
                            capture_timeline: animations::capture_flash::animation()
                                .begin_animation(),
                        }
                    }
                    _ => (),
                }
                Task::none()
            }
            MainAppMessage::Tick => match &mut self.state {
                MainAppState::CapturePhotosPrepare { ready_timeline } => {
                    if ready_timeline.update().is_completed() {
                        self.state = MainAppState::CapturePhotos {
                            current: 0,
                            state: CapturePhotosState::Countdown {
                                current: 3,
                                countdown_timeline: animations::countdown_circle::animation()
                                    .begin_animation(),
                            },
                        }
                    };
                    Task::none()
                }
                MainAppState::CapturePhotos { state, current } => match state {
                    CapturePhotosState::Countdown {
                        current,
                        countdown_timeline,
                    } => {
                        if countdown_timeline.update().is_completed() {
                            *current -= 1;
                            if *current == 0 {
                                *state = CapturePhotosState::Capture {
                                    capture_timeline: animations::capture_flash::animation()
                                        .to_timeline(),
                                };
                                return Task::done(MainAppMessage::CaptureStill);
                            } else {
                                *countdown_timeline =
                                    animations::countdown_circle::animation().begin_animation();
                            }
                        };
                        Task::none()
                    }
                    CapturePhotosState::Capture { capture_timeline } => {
                        if capture_timeline.update().is_completed() {
                            let last_photo = self
                                .captured_photos
                                .last()
                                .expect("capture didn't complete")
                                .clone();
                            *state = CapturePhotosState::Preview {
                                preview_timeline: animations::capture_preview::animation()
                                    .begin_animation(),
                                captured_handle: Handle::from_rgba(
                                    last_photo.width(),
                                    last_photo.height(),
                                    last_photo.into_raw(),
                                ),
                            }
                        };
                        Task::none()
                    }
                    CapturePhotosState::Preview {
                        preview_timeline, ..
                    } => {
                        if preview_timeline.update().is_completed() {
                            *current += 1;
                            if *current < PHOTO_COUNT {
                                *state = CapturePhotosState::Countdown {
                                    current: 3,
                                    countdown_timeline: animations::countdown_circle::animation()
                                        .begin_animation(),
                                };
                                Task::none()
                            } else {
                                self.state = MainAppState::Uploading {
                                    progress_timeline: anim::Options::new(0.0, 0.6)
                                        .duration(Duration::from_millis(8000))
                                        .easing(
                                            anim::easing::cubic_ease()
                                                .mode(anim::easing::EasingMode::Out),
                                        )
                                        .begin_animation(),
                                };
                                let old = self.captured_photos.drain(..).collect::<Vec<_>>();
                                self.previews.clear();
                                for photo in &old {
                                    self.previews.push(iced::widget::image::Handle::from_rgba(
                                        photo.width(),
                                        photo.height(),
                                        photo.as_raw().clone(),
                                    ));
                                }
                                let future = server_backend.upload_photo(old[0].clone(), old);
                                Task::perform(future, |result| {
                                    MainAppMessage::Uploaded(result.map_err(|x| x.to_string()))
                                })
                            }
                        } else {
                            Task::none()
                        }
                    }
                },
                MainAppState::Uploading { progress_timeline } => {
                    if progress_timeline.update().is_completed() && progress_timeline.value() == 1.0
                    {
                        self.state = MainAppState::EditPrintUpsellBanner {
                            progress_timeline: anim::Options::new(0.0, 1.0)
                                .duration(Duration::from_millis(4000))
                                .easing(anim::easing::linear())
                                .begin_animation(),
                            template_preview_timeline: animations::upsell_templates::animation()
                                .begin_animation(),
                            template_index: 0,
                        }
                    }
                    Task::none()
                }
                MainAppState::EditPrintUpsellBanner {
                    progress_timeline,
                    template_preview_timeline,
                    ref mut template_index,
                } => {
                    if template_preview_timeline.update().is_completed() {
                        *template_index += 1;
                        *template_preview_timeline =
                            animations::upsell_templates::animation().begin_animation()
                    }
                    if progress_timeline.update().is_completed() {
                        self.state = MainAppState::PaymentRequired { show_error: false };
                    }
                    Task::none()
                }
                _ => Task::none(),
            },
            MainAppMessage::Uploaded(result) => match self.state {
                MainAppState::Uploading {
                    ref mut progress_timeline,
                } => match result {
                    Ok(_) => {
                        *progress_timeline = anim::Options::new(progress_timeline.value(), 1.0)
                            .duration(Duration::from_millis(2000))
                            .easing(
                                anim::easing::cubic_ease().mode(anim::easing::EasingMode::InOut),
                            )
                            .begin_animation();
                        Task::none()
                    }
                    Err(err) => {
                        panic!("something went wrong: {}", err)
                    }
                },
                _ => Task::none(),
            },
            MainAppMessage::KeyReleased(key) => match &mut self.state {
                MainAppState::PaymentRequired { .. } => match key {
                    KeyMessage::Up => Task::none(),
                    KeyMessage::Down => Task::none(),
                    KeyMessage::Space => {
                        self.state = MainAppState::CapturePhotosPrepare {
                            ready_timeline: animations::ready::animation().begin_animation(),
                        };
                        Task::none()
                    }
                },
                MainAppState::Preview => {
                    self.state = MainAppState::CapturePhotosPrepare {
                        ready_timeline: animations::ready::animation().begin_animation(),
                    };
                    Task::none()
                }
                MainAppState::EditPrintUpsellBanner {
                    progress_timeline, ..
                } => {
                    *progress_timeline = anim::Options::new(progress_timeline.value(), 1.0)
                        .duration(Duration::from_millis(1000))
                        .easing(anim::easing::cubic_ease().mode(anim::easing::EasingMode::InOut))
                        .begin_animation();
                    Task::none()
                }
                _ => Task::none(),
            },
        }
    }

    pub fn view<'a>(&'a self, _server_backend: &'a S) -> Element<'a, MainAppMessage<S>> {
        iced::widget::stack([
            self.feed
                .view()
                .content_fit(
                    if matches!(
                        self.state,
                        MainAppState::CapturePhotosPrepare { .. }
                            | MainAppState::CapturePhotos { .. }
                            | MainAppState::Preview
                    ) {
                        ContentFit::Contain
                    } else {
                        ContentFit::Cover
                    },
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            match &self.state {
                MainAppState::PaymentRequired { show_error } => title_overlay(
                    container(
                        container(
                            column([
                                iced::widget::text("Shrimp Game Check-in")
                                    .size(42)
                                    .style(|theme: &iced::Theme| iced::widget::text::Style {
                                        color: Some(theme.extended_palette().background.base.text),
                                    })
                                    .into(),
                                vertical_space().height(6).into(),
                                scrollable(column([]).spacing(8))
                                    .id(Id::new("team_scrollable"))
                                    .into(),
                                vertical_space().height(12).into(),
                                if *show_error {
                                    column([
                                        vertical_space().height(12).into(),
                                        container(column([iced::widget::text(
                                            "An error occurred.",
                                        )
                                        .size(12)
                                        .into()]))
                                        .style(|theme: &iced::Theme| container::Style {
                                            border: iced::Border::default().rounded(4.0).color(
                                                theme.extended_palette().danger.strong.color,
                                            ),
                                            background: Some(
                                                theme.extended_palette().danger.weak.color.into(),
                                            ),
                                            text_color: Some(
                                                theme.extended_palette().danger.weak.text,
                                            ),
                                            ..Default::default()
                                        })
                                        .padding(8)
                                        .into(),
                                    ])
                                    .into()
                                } else {
                                    Space::new(0, 0).into()
                                },
                            ])
                            .align_x(Alignment::Center),
                        )
                        .max_width(780)
                        .padding(18)
                        .style(|theme: &iced::Theme| container::Style {
                            border: iced::Border::default().rounded(28),
                            background: Some(
                                theme
                                    .extended_palette()
                                    .background
                                    .strong
                                    .color
                                    .scale_alpha(0.2)
                                    .into(),
                            ),
                            ..Default::default()
                        }),
                    )
                    .center(Length::Fill),
                    false,
                )
                .into(),
                MainAppState::Preview => title_overlay(
                    column([
                        title_text("Entering the games requires creating visual identification.")
                            .into(),
                        supporting_text("Press space to create your official photograph.").into(),
                        vertical_space().height(12.0).into(),
                    ]),
                    true,
                ),
                MainAppState::CapturePhotosPrepare { ready_timeline } => {
                    animations::ready::view(ready_timeline.value()).into()
                }
                MainAppState::CapturePhotos { current: _, state } => match state {
                    CapturePhotosState::Countdown {
                        current,
                        countdown_timeline,
                    } => animations::countdown_circle::view(*current, countdown_timeline.value())
                        .into(),
                    CapturePhotosState::Capture { capture_timeline } => {
                        animations::capture_flash::view(capture_timeline.value()).into()
                    }
                    CapturePhotosState::Preview {
                        preview_timeline,
                        captured_handle,
                    } => {
                        animations::capture_preview::view(captured_handle, preview_timeline.value())
                            .into()
                    }
                },
                MainAppState::Uploading { progress_timeline } => title_overlay(
                    iced::widget::column([
                        container(
                            loading_spinners::Circular::new()
                                .size(96.0)
                                .bar_height(10.0)
                                .easing(&loading_spinners::easing::STANDARD_DECELERATE),
                        )
                        .center(Length::Fill)
                        .into(),
                        title_text("We're uploading your team photos now.").into(),
                        supporting_text("You may proceed shortly.").into(),
                        vertical_space().height(12.0).into(),
                        progress_bar(0.0..=1.0, progress_timeline.value())
                            .height(8.0)
                            .into(),
                    ]),
                    false,
                )
                .into(),
                MainAppState::EditPrintUpsellBanner {
                    progress_timeline,
                    template_preview_timeline,
                    template_index,
                } => title_overlay(
                    column([
                        animations::upsell_templates::view(
                            &self.previews[template_index % self.previews.len()],
                            template_preview_timeline.value(),
                        )
                        .into(),
                        title_text("Test")
                            .shaping(iced::widget::text::Shaping::Advanced)
                            .into(),
                        supporting_text("The team listed above has been confirmed. Proceed.")
                            .into(),
                        vertical_space().height(12.0).into(),
                        progress_bar(0.0..=1.0, progress_timeline.value())
                            .height(4.0)
                            .into(),
                    ]),
                    false,
                )
                .into(),
            },
        ])
        .into()
    }
}
