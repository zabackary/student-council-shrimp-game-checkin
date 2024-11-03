use std::time::Duration;

use anim::{easing, Animatable};
use iced::{
    widget::{column, container, image, image::Handle, responsive, vertical_space, Container},
    Length,
};

use crate::frontend::main_app::PHOTO_ASPECT_RATIO;

use super::LENGTH_DIVISOR;

pub const ANIMATION_LENGTH: u64 = 4000 / LENGTH_DIVISOR;

#[derive(Debug, Clone, Copy, Animatable)]
pub struct AnimationState {
    opacity: f32,
    offset_scale: f32,
    width_scale: f32,
    background_opacity: f32,
}

pub fn animation() -> impl anim::Animation<Item = AnimationState> {
    anim::builder::key_frames([
        anim::KeyFrame::new(AnimationState {
            opacity: 0.0,
            offset_scale: 1.0,
            width_scale: 0.4,
            background_opacity: 0.0,
        })
        .by_percent(0.0),
        anim::KeyFrame::new(AnimationState {
            opacity: 1.0,
            offset_scale: 0.0,
            width_scale: 1.0,
            background_opacity: 0.9,
        })
        .easing(easing::cubic_ease().mode(easing::EasingMode::Out))
        .by_percent(0.2),
        anim::KeyFrame::new(AnimationState {
            opacity: 1.0,
            offset_scale: 0.0,
            width_scale: 1.0,
            background_opacity: 0.9,
        })
        .by_percent(0.7),
        anim::KeyFrame::new(AnimationState {
            opacity: 0.0,
            offset_scale: 0.0,
            width_scale: 1.0,
            background_opacity: 0.0,
        })
        .easing(easing::cubic_ease().mode(easing::EasingMode::In))
        .by_duration(Duration::from_millis(ANIMATION_LENGTH)),
    ])
}

pub fn view<'a, Message: 'static>(
    handle: &'a Handle,
    animation_state: AnimationState,
) -> Container<'a, Message> {
    container(responsive(move |size| {
        let image_width = animation_state.width_scale * size.width * 0.8;
        let image_height = image_width / PHOTO_ASPECT_RATIO;

        let remaining_vertical_space = size.height - image_height;

        container(column([
            vertical_space()
                .height(remaining_vertical_space * animation_state.offset_scale)
                .into(),
            image(handle)
                .opacity(animation_state.opacity)
                .width(image_width)
                .height(image_height)
                .into(),
        ]))
        .center(Length::Fill)
        .into()
    }))
}
