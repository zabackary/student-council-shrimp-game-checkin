use std::time::Duration;

use anim::{easing, Animatable};
use iced::{
    widget::{container, Container},
    Color, Length,
};

pub const ANIMATION_LENGTH: u64 = 400;

#[derive(Debug, Clone, Copy, Animatable)]
pub struct AnimationState {
    opacity: f32,
}

pub fn animation() -> impl anim::Animation<Item = AnimationState> {
    anim::builder::key_frames([
        anim::KeyFrame::new(AnimationState { opacity: 1.0 }).by_percent(0.0),
        anim::KeyFrame::new(AnimationState { opacity: 0.0 })
            .easing(easing::cubic_ease().mode(easing::EasingMode::Out))
            .by_duration(Duration::from_millis(ANIMATION_LENGTH)),
    ])
}

pub fn view<Message>(animation_state: AnimationState) -> Container<'static, Message> {
    container("")
        .style(move |_| container::Style {
            background: Some(Color::WHITE.scale_alpha(animation_state.opacity).into()),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
}
