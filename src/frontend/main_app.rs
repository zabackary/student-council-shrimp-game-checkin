use std::time::Duration;

use anim::Animation;
use iced::{
    widget::{
        column, container, horizontal_space, image::Handle, progress_bar, row, vertical_space,
        Space,
    },
    Alignment, Color, ContentFit, Element, Length, Task,
};
use image::RgbaImage;

use crate::{backend::render_take::render_take, AppPage, KeyMessage, PhotoBoothMessage};

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
    EmailEntry,
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
    strip: Option<RgbaImage>,
    strip_handle: Option<Handle>,
    logo_handle: Handle,
    emails: Vec<String>,
    upload_handle: Option<S::UploadHandle>,
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
                logo_handle: Handle::from_bytes(
                    include_bytes!("../../assets/75thAnniversaryLogo.jpg").to_vec(),
                ),
                strip: None,
                strip_handle: None,

                emails: Vec::new(),
                upload_handle: None,
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
                                        .duration(Duration::from_millis(12000))
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
                                self.strip = Some(render_take(old.clone()));
                                self.strip_handle = Some(Handle::from_rgba(
                                    self.strip.as_ref().unwrap().width(),
                                    self.strip.as_ref().unwrap().height(),
                                    self.strip.as_ref().unwrap().as_raw().clone(),
                                ));
                                let future = server_backend
                                    .upload_photo(self.strip.as_ref().unwrap().clone(), old);
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
                                .duration(Duration::from_millis(
                                    animations::upsell_templates::ANIMATION_LENGTH,
                                ))
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
                        self.state = MainAppState::EmailEntry;
                    }
                    Task::none()
                }
                _ => Task::none(),
            },
            MainAppMessage::Uploaded(result) => match self.state {
                MainAppState::Uploading {
                    ref mut progress_timeline,
                } => match result {
                    Ok(res) => {
                        self.upload_handle = Some(res);
                        *progress_timeline = anim::Options::new(progress_timeline.value(), 1.0)
                            .duration(Duration::from_millis(2000))
                            .easing(
                                anim::easing::cubic_ease().mode(anim::easing::EasingMode::InOut),
                            )
                            .begin_animation();
                        Task::none()
                    }
                    Err(err) => {
                        self.state = MainAppState::PaymentRequired { show_error: true };
                        log::error!("Error uploading photos: {}", err);
                        Task::none()
                    }
                },
                _ => Task::none(),
            },
            MainAppMessage::KeyReleased(key) => match &mut self.state {
                MainAppState::PaymentRequired { .. } => match key {
                    KeyMessage::Up => Task::none(),
                    KeyMessage::Down => Task::none(),
                    KeyMessage::Space => {
                        self.state = MainAppState::Preview;
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
                                iced::widget::text("CAJ 75th Anniversary Photo Booth")
                                    .size(42)
                                    .style(|theme: &iced::Theme| iced::widget::text::Style {
                                        color: Some(theme.extended_palette().primary.base.text),
                                    })
                                    .into(),
                                vertical_space().height(6).into(),
                                iced::widget::image(self.logo_handle.clone())
                                    .width(500)
                                    .height(500)
                                    .content_fit(ContentFit::Contain)
                                    .into(),
                                vertical_space().height(6).into(),
                                iced::widget::text("Press [SPACE] to get started.")
                                    .size(24)
                                    .into(),
                                    vertical_space().height(12).into(),
                                    iced::widget::text("By using this photo booth, you consent to having your photos uploaded and processed by our servers and Google Drive.")
                                        .size(18)
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
                            background: Some(theme.extended_palette().primary.base.color.into()),
                            text_color: Some(Color::from_rgb8(0xff, 0xff, 0xff)),
                            ..Default::default()
                        }),
                    )
                    .center(Length::Fill),
                    false,
                )
                .into(),
                MainAppState::Preview => title_overlay(
                    column([
                        title_text("Get read to take your pictures").into(),
                        supporting_text("Press [SPACE] to start when you're ready.").into(),
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
                        title_text("We're uploading your photos now.").into(),
                        supporting_text("You'll be able to enter your emails in a second.").into(),
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
                        title_text("Your photos are ready!").into(),
                        supporting_text("On the next screen, enter your emails.").into(),
                        vertical_space().height(12.0).into(),
                        progress_bar(0.0..=1.0, progress_timeline.value())
                            .height(4.0)
                            .into(),
                    ]),
                    false,
                )
                .into(),
                MainAppState::EmailEntry => title_overlay(
                    row([
                        column([
                            title_text("Enter your email addresses").into(),
                            supporting_text("Press [ENTER] to add an email.").into(),
                            vertical_space().height(12.0).into(),
                            container(
                                column(self.emails.iter().map(|email| {
                                    iced::widget::text(email)
                                        .size(24)
                                        .style(|theme: &iced::Theme| iced::widget::text::Style {
                                            color: Some(
                                                theme.extended_palette().background.base.text,
                                            ),
                                        })
                                        .into()
                                }))
                                .align_x(Alignment::Center),
                            )
                            .center(Length::Fill)
                            .into(),
                        ])
                        .into(),
                        horizontal_space().width(12.0).into(),
                        iced::widget::image(self.strip_handle.as_ref().unwrap().clone())
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .content_fit(ContentFit::Contain)
                            .into(),
                    ]),
                    false,
                ),
            },
        ])
        .into()
    }
}
