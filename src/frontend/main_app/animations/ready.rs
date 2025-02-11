use std::time::Duration;

use anim::{easing, Animatable};
use iced::{
    widget::{column, container, text, vertical_space, Container},
    Border, Length,
};

use super::LENGTH_DIVISOR;

pub const ANIMATION_LENGTH: u64 = 3000 / LENGTH_DIVISOR;

#[derive(Debug, Clone, Copy, Animatable)]
pub struct AnimationState {
    opacity: f32,
    text_size: f32,
    offset: f32,
}

const TEXT_SIZE: f32 = 60.0;

pub fn animation() -> impl anim::Animation<Item = AnimationState> {
    anim::builder::key_frames([
        anim::KeyFrame::new(AnimationState {
            opacity: 0.0,
            text_size: TEXT_SIZE * 0.8,
            offset: 200.0,
        })
        .by_percent(0.0),
        anim::KeyFrame::new(AnimationState {
            opacity: 1.0,
            text_size: TEXT_SIZE,
            offset: 0.0,
        })
        .easing(easing::cubic_ease().mode(easing::EasingMode::Out))
        .by_percent(0.4),
        anim::KeyFrame::new(AnimationState {
            opacity: 1.0,
            text_size: TEXT_SIZE,
            offset: 0.0,
        })
        .by_percent(0.8),
        anim::KeyFrame::new(AnimationState {
            opacity: 0.0,
            text_size: TEXT_SIZE * 0.8,
            offset: 200.0,
        })
        .easing(easing::cubic_ease().mode(easing::EasingMode::In))
        .by_duration(Duration::from_millis(ANIMATION_LENGTH)),
    ])
}

pub fn view<Message: 'static>(animation_state: AnimationState) -> Container<'static, Message> {
    container(column([
        vertical_space().height(animation_state.offset).into(),
        container(text(format!("Smile for the photograph.")).size(animation_state.text_size))
            .style(move |theme: &iced::Theme| container::Style {
                text_color: Some(
                    theme
                        .extended_palette()
                        .primary
                        .weak
                        .text
                        .scale_alpha(animation_state.opacity),
                ),
                background: Some(
                    theme
                        .extended_palette()
                        .primary
                        .weak
                        .color
                        .scale_alpha(animation_state.opacity)
                        .into(),
                ),
                border: Border {
                    radius: 9999.0.into(),
                    ..Default::default()
                },
                shadow: Default::default(),
            })
            .padding(24)
            .into(),
    ]))
    .center(Length::Fill)
}
