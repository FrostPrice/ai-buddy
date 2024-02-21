use async_openai::types::{CreateMessageRequest, MessageContent, MessageObject};

use crate::Result;

// Message Constructors
pub fn user_msg(content: impl Into<String>) -> CreateMessageRequest {
    CreateMessageRequest {
        role: "user".to_string(),
        content: content.into(),
        ..Default::default()
    }
}

// Content Extractor
pub fn get_text_content(msg: MessageObject) -> Result<String> {
    // Get first content item
    let msg_content = msg
        .content
        .into_iter()
        .next()
        .ok_or_else(|| "No message content found".to_string())?;

    // Get the text
    let txt = match msg_content {
        MessageContent::Text(text) => text.text.value,
        MessageContent::ImageFile(_) => return Err("Message image not supported yet".into()),
    };

    Ok(txt)
}
