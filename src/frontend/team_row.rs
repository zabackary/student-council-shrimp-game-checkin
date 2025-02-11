use iced::{
    widget::{container, horizontal_space, row, text},
    Alignment, Border, Color, Element, Font, Length,
};

pub fn team_row<'a, Message: 'a>(
    team_name: &'a str,
    highlight: bool,
    checked_in: bool,
) -> Element<'a, Message> {
    container(
        row([
            text(team_name)
                .size(36)
                .shaping(text::Shaping::Advanced)
                .into(),
            horizontal_space().into(),
            text(if checked_in { "✅" } else { "❌" })
                .font(Font::with_name("Noto Color Emoji"))
                .size(24)
                .into(),
        ])
        .align_y(Alignment::Center)
        .spacing(20),
    )
    .style(move |theme: &iced::Theme| container::Style {
        background: Some(
            if highlight {
                theme.extended_palette().primary.base.color.scale_alpha(0.3)
            } else if checked_in {
                Color::TRANSPARENT
            } else {
                theme
                    .extended_palette()
                    .background
                    .strong
                    .color
                    .scale_alpha(0.2)
            }
            .into(),
        ),
        border: if highlight {
            Border {
                color: theme.extended_palette().primary.base.color.scale_alpha(0.8),
                width: 2.0,
                radius: 8.0.into(),
            }
        } else {
            Border {
                color: theme
                    .extended_palette()
                    .background
                    .base
                    .color
                    .scale_alpha(if checked_in { 0.3 } else { 0.8 }),
                width: 2.0,
                radius: 32.0.into(),
            }
        },
        text_color: Some(theme.extended_palette().background.base.text.scale_alpha(
            if checked_in {
                0.3
            } else if highlight {
                1.0
            } else {
                0.6
            },
        )),
        ..Default::default()
    })
    .height(64.0)
    .padding(8.0)
    .width(Length::Fill)
    .into()
}
