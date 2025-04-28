use iced::{widget::Container, Element};

/// A small overlay for displaying status messages.
///
/// Text should be passed with a font size of 24.
pub fn status_overlay<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
) -> Container<'a, Message> {
    iced::widget::container(iced::widget::container(content).padding(12).style(
        move |theme: &iced::Theme| iced::widget::container::Style {
            text_color: Some(theme.extended_palette().primary.weak.text),
            background: Some(theme.extended_palette().primary.weak.color.into()),
            border: iced::Border {
                radius: 9999.0.into(),
                ..Default::default()
            },
            shadow: Default::default(),
        },
    ))
    .center(iced::Length::Fill)
    .align_x(iced::Alignment::Start)
    .align_y(iced::Alignment::End)
    .padding(24)
}
