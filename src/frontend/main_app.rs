use anim::Animation;
use iced::{
    widget::{container, image::Handle, text, Container},
    ContentFit, Element, Length, Task,
};
use image::RgbaImage;

use crate::{AppPage, PhotoBoothMessage};

use super::{
    camera_feed::{CameraFeed, CameraFeedOptions},
    title_overlay::{title_overlay, title_text},
};

mod animations;

const PHOTO_ASPECT_RATIO: f32 = 3.0 / 2.0;

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
    PaymentRequired,
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
        animation_timeline: anim::Timeline<f32>,
    },
}

#[derive(Debug, Clone)]
pub enum MainAppMessage {
    Camera(super::camera_feed::CameraMessage),
    Tick,
    SpaceReleased,
    CaptureStill,
}

pub struct MainApp<C: crate::backend::cameras::CameraBackend + 'static> {
    feed: CameraFeed<C::Camera>,
    state: MainAppState,
    captured_photos: Vec<RgbaImage>,
    pub new_page: Option<Box<(AppPage<C>, Task<PhotoBoothMessage<C>>)>>,
}

impl<C: crate::backend::cameras::CameraBackend + 'static> MainApp<C> {
    pub fn new(feed: CameraFeed<C::Camera>) -> Self {
        Self {
            feed,
            state: MainAppState::Preview,
            new_page: None,
            captured_photos: Vec::with_capacity(4),
        }
    }

    pub fn update(&mut self, message: MainAppMessage) -> Task<MainAppMessage> {
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
            MainAppMessage::Tick => {
                match &mut self.state {
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
                        }
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
                            }
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
                            }
                        }
                        CapturePhotosState::Preview {
                            preview_timeline, ..
                        } => {
                            if preview_timeline.update().is_completed() {
                                *current += 1;
                                if *current < 3 {
                                    *state = CapturePhotosState::Countdown {
                                        current: 3,
                                        countdown_timeline:
                                            animations::countdown_circle::animation()
                                                .begin_animation(),
                                    }
                                } else {
                                    todo!()
                                }
                            }
                        }
                    },
                    MainAppState::Uploading { progress_timeline } => {
                        progress_timeline.update();
                    }
                    MainAppState::EditPrintUpsellBanner { animation_timeline } => {
                        animation_timeline.update();
                    }
                    _ => {}
                };
                Task::none()
            }
            MainAppMessage::SpaceReleased => {
                match &mut self.state {
                    MainAppState::Preview => {
                        self.state = MainAppState::CapturePhotosPrepare {
                            ready_timeline: animations::ready::animation().begin_animation(),
                        };
                    }
                    _ => {}
                };
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<MainAppMessage> {
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
                MainAppState::PaymentRequired => {
                    title_overlay(title_text("Photo Booth, only 1 ticket!")).into()
                }
                MainAppState::Preview => {
                    title_text("Press [SPACE] to start taking pictures!").into()
                }
                MainAppState::CapturePhotosPrepare { ready_timeline } => {
                    animations::ready::view(ready_timeline.value()).into()
                }
                MainAppState::CapturePhotos { current, state } => match state {
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
                MainAppState::Uploading { progress_timeline } => {
                    title_overlay(title_text("Your photos are being uploaded.")).into()
                }
                MainAppState::EditPrintUpsellBanner { animation_timeline } => title_overlay(
                    title_text("Edit and download your photo at the nearby kiosk"),
                )
                .into(),
            },
        ])
        .into()
    }
}
