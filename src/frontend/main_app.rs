use std::time::Duration;

use anim::Animation;
use iced::{
    widget::{column, container, image::Handle, progress_bar, row, vertical_space, Space},
    Alignment, ContentFit, Element, Length, Task,
};
use image::RgbaImage;

use crate::{AppPage, PhotoBoothMessage};

use super::{
    camera_feed::{CameraFeed, CameraFeedOptions},
    loading_spinners,
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
    SpaceReleased,
    CaptureStill,
    Uploaded(Result<S::UploadHandle, String>),
    PreviewDownloaded(Result<Vec<RgbaImage>, String>),
    IsUnlockedResponse(Result<Option<bool>, String>),
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
    pub fn new(feed: CameraFeed<C::Camera>) -> Self {
        Self {
            feed,
            state: MainAppState::PaymentRequired { show_error: false },
            new_page: None,
            captured_photos: Vec::with_capacity(PHOTO_COUNT),
            previews: Vec::with_capacity(PHOTO_COUNT),
        }
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
                                let future = server_backend
                                    .upload_photo(old.into_iter().next().unwrap(), -1); // TODO: get team number
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
                                .duration(Duration::from_millis(20000))
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
                    Ok(handle) => {
                        *progress_timeline = anim::Options::new(progress_timeline.value(), 0.8)
                            .duration(Duration::from_millis(2000))
                            .easing(
                                anim::easing::cubic_ease().mode(anim::easing::EasingMode::InOut),
                            )
                            .begin_animation();
                        Task::done(MainAppMessage::<S>::PreviewDownloaded(
                            Ok(Vec::new()), // FIXME: implement this
                        ))
                    }
                    Err(err) => {
                        panic!("something went wrong: {}", err)
                    }
                },
                _ => Task::none(),
            },
            MainAppMessage::PreviewDownloaded(result) => match self.state {
                MainAppState::Uploading {
                    ref mut progress_timeline,
                } => {
                    match result {
                        Ok(handle) => {
                            self.previews = handle
                                .into_iter()
                                .map(|img| {
                                    iced::widget::image::Handle::from_rgba(
                                        img.width(),
                                        img.height(),
                                        img.into_raw(),
                                    )
                                })
                                .collect();
                            *progress_timeline = anim::Options::new(progress_timeline.value(), 1.0)
                                .duration(Duration::from_millis(1000))
                                .easing(
                                    anim::easing::cubic_ease()
                                        .mode(anim::easing::EasingMode::InOut),
                                )
                                .begin_animation();
                        }
                        Err(err) => panic!("something went wrong: {}", err),
                    }
                    Task::none()
                }
                _ => Task::none(),
            },
            MainAppMessage::IsUnlockedResponse(result) => match self.state {
                MainAppState::PaymentRequired { ref mut show_error } => match result {
                    Ok(maybe_ok) => {
                        if let Some(is_ok) = maybe_ok {
                            if is_ok {
                                self.state = MainAppState::Preview;
                            } else {
                                *show_error = true;
                            }
                        } else {
                            self.state = MainAppState::Preview;
                        }
                        Task::none()
                    }
                    Err(err) => {
                        panic!("failed to update is_unlocked: {}", err);
                    }
                },
                _ => Task::none(),
            },
            MainAppMessage::SpaceReleased => match &mut self.state {
                MainAppState::PaymentRequired { .. } => {
                    Task::done(MainAppMessage::IsUnlockedResponse(Ok(Some(true))))
                }
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

    pub fn view<'a>(&'a self, server_backend: &'a S) -> Element<'a, MainAppMessage<S>> {
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
                        title_text("Press the space key to start taking pictures!").into(),
                        supporting_text("スペースキーを押すと、撮影が開始されます。").into(),
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
                        title_text("Your photos are being uploaded.").into(),
                        supporting_text("写真がアップロードされています。").into(),
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
                        title_text("Edit and download your photos right outside").into(),
                        supporting_text("入口にお戻りいただくと写真の編集やダウンロードが可能です")
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
