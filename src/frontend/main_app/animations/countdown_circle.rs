use std::time::Duration;

use anim::{easing, Animatable};
use iced::{
    widget::{container, text, Container},
    Border, Length,
};

use super::LENGTH_DIVISOR;

pub const ANIMATION_LENGTH: u64 = 1000 / LENGTH_DIVISOR;

#[derive(Debug, Clone, Copy, Animatable)]
pub struct AnimationState {
    opacity: f32,
    text_size: f32,
}

const MIN_TEXT_SIZE: f32 = f32::MIN_POSITIVE;
const TEXT_SIZE: f32 = 60.0;

pub fn animation() -> impl anim::Animation<Item = AnimationState> {
    anim::builder::key_frames([
        anim::KeyFrame::new(AnimationState {
            opacity: 0.0,
            text_size: MIN_TEXT_SIZE,
        })
        .by_percent(0.0),
        anim::KeyFrame::new(AnimationState {
            opacity: 1.0,
            text_size: TEXT_SIZE,
        })
        .easing(easing::cubic_ease().mode(easing::EasingMode::Out))
        .by_percent(0.4),
        anim::KeyFrame::new(AnimationState {
            opacity: 1.0,
            text_size: TEXT_SIZE,
        })
        .by_percent(0.8),
        anim::KeyFrame::new(AnimationState {
            opacity: 9.0,
            text_size: MIN_TEXT_SIZE,
        })
        .easing(easing::cubic_ease().mode(easing::EasingMode::In))
        .by_duration(Duration::from_millis(ANIMATION_LENGTH)),
    ])
}

pub fn view<Message: 'static>(
    value: usize,
    animation_state: AnimationState,
) -> Container<'static, Message> {
    container(
        container(text(format!("{value}")).size(animation_state.text_size))
            .padding(24)
            .style(move |theme: &iced::Theme| container::Style {
                text_color: Some(
                    theme
                        .extended_palette()
                        .primary
                        .strong
                        .text
                        .scale_alpha(animation_state.opacity),
                ),
                background: Some(
                    theme
                        .extended_palette()
                        .primary
                        .strong
                        .color
                        .scale_alpha(animation_state.opacity)
                        .into(),
                ),
                border: Border {
                    radius: 9999.0.into(),
                    ..Default::default()
                },
                shadow: Default::default(),
            }),
    )
    .center(Length::Fill)
}
