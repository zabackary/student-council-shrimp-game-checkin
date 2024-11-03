use iced::{
    gradient::Linear,
    widget::{container, text, Text},
    Alignment, Background, Color, Element, Length, Radians,
};

pub fn title_overlay<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    minimize_overlay: bool,
) -> Element<'a, Message> {
    container(content)
        .style(move |theme: &iced::Theme| {
            container::background(Background::Gradient(if minimize_overlay {
                iced::Gradient::Linear(
                    Linear::new(Radians::PI)
                        .add_stop(0.0, Color::TRANSPARENT)
                        .add_stop(0.4, Color::TRANSPARENT)
                        .add_stop(1.0, theme.extended_palette().background.base.color),
                )
            } else {
                iced::Gradient::Linear(
                    Linear::new(Radians::PI)
                        .add_stop(
                            0.0,
                            theme
                                .extended_palette()
                                .background
                                .base
                                .color
                                .scale_alpha(0.7),
                        )
                        .add_stop(
                            0.4,
                            theme
                                .extended_palette()
                                .background
                                .base
                                .color
                                .scale_alpha(0.7),
                        )
                        .add_stop(1.0, theme.extended_palette().background.base.color),
                )
            }))
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .align_y(Alignment::End)
        .align_x(Alignment::Center)
        .into()
}

pub fn title_text(content: &str) -> Text {
    text(content)
        .style(|theme: &iced::Theme| text::Style {
            color: Some(theme.extended_palette().background.base.text),
        })
        .size(42)
        .wrapping(text::Wrapping::None)
        .align_x(Alignment::Center)
        .width(Length::Fill)
}

pub fn supporting_text(content: &str) -> Text {
    text(content)
        .style(|theme: &iced::Theme| text::Style {
            color: Some(
                theme
                    .extended_palette()
                    .background
                    .base
                    .text
                    .scale_alpha(0.6),
            ),
        })
        .size(32)
        .wrapping(text::Wrapping::None)
        .align_x(Alignment::Center)
        .width(Length::Fill)
}
