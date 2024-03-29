use console::{style, Style, StyledObject};
use dialoguer::{theme::ColorfulTheme, Input};

use crate::Result;

// Prompts
pub fn prompt(text: &str) -> Result<String> {
    // let theme = ColorfulTheme::default();
    let theme = ColorfulTheme {
        prompt_style: Style::new().for_stderr().color256(45),
        prompt_prefix: style("?".to_string()).color256(45).for_stderr(),
        ..ColorfulTheme::default()
    };

    let input = Input::with_theme(&theme);
    let res = input.with_prompt(text).interact_text()?;

    Ok(res)
}

// Icons
pub fn icon_check() -> StyledObject<&'static str> {
    style("✔").green()
}

pub fn icon_uploading() -> StyledObject<&'static str> {
    style("↥").yellow()
}

pub fn icon_uploaded() -> StyledObject<&'static str> {
    style("↥").green()
}

pub fn icon_deleted_ok() -> StyledObject<&'static str> {
    style("⌫").green()
}

pub fn icon_err() -> StyledObject<&'static str> {
    style("✗").red()
}

pub fn icon_res() -> StyledObject<&'static str> {
    style("➤").color256(45)
}

// Text Output
pub fn text_res(text: String) -> StyledObject<String> {
    style(text).bright()
}
