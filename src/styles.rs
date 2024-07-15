use iced::{Background, Border, Color, Theme};
use iced::widget::{button, Button};
use iced::application::Appearance;
use iced::widget::button::Style;
use crate::Message;

pub fn news_pages_switch_button_style(theme: &Theme) -> Box<dyn Fn(&Theme, button::Status) -> button::Style> {
    Box::new(move |theme, _status| {
        button::Style {
            background: Some(Background::Color(Color::from_rgb8(255, 165, 0))),
            text_color: Color::from_rgb8(0, 0, 139),
            border: Border::rounded(200),
            shadow: Default::default(),
        }
    })
}

impl StyleSheet for Button<'_, Message> {
    type Style = ();

    fn appearance(&self, _style: &Self::Style) -> Appearance {
        //let palette = self.extended_palette();

        Appearance {
            background_color: Color::from_rgb(255.,255.,255.),
            text_color: Color::from_rgb(0.,0.,0.)
        }
    }
}

/// A set of rules that dictate the style of an indicator.
pub trait StyleSheet {
    /// The supported style of the [`StyleSheet`].
    type Style: Default;

    /// Produces the active [`Appearance`] of a indicator.
    fn appearance(&self, style: &Self::Style) -> Appearance;
}

pub fn news_pages_selected_button_style(theme: &Theme) -> Box<dyn Fn(&Theme, button::Status) -> button::Style> {
    Box::new(move |theme, status| {

        use iced::{Background, Color, Border, Vector};

        let base_color = Color::from_rgb8(255, 0, 0); // Brighter yellow for highlight
        let hovered_color = Color::from_rgb8(255, 0, 0);; //.darken(0.1); // Slightly darker when hovered

        let base_style = Style {
            background: Some(Background::Color(base_color)),
            text_color: Color::BLACK, // Ensuring good contrast
            border: Border::rounded(10), // More pronounced rounded effect
            shadow: iced::Shadow {
                offset: Vector::new(1.0, 2.0),
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
                blur_radius: 5.0,
            },
            ..Style::default()
        };

        match status {
            button::Status::Hovered => Style {
                background: Some(Background::Color(hovered_color)),
                ..base_style
            },
            _ => base_style,
        }
    })
}

// Implement a transparent button style
pub fn transparent_button_hyperlink_style(theme: &Theme) -> Box<dyn Fn(&Theme, button::Status) -> button::Style>
{
    Box::new(move |theme, _status| {
        button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: Color::from_rgb8(255, 204, 0),
            border: Border::rounded(0),
            shadow: Default::default(),
        }
    })
}
