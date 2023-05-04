#![allow(dead_code)]
#![allow(unused)]

use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub async fn make_request<'a>(api_key: &str, messages: Vec<ChatMessage>) -> Result<ChatResponse> {
    let client = Client::new();
    let url = "https://api.openai.com/v1/chat/completions";
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
    );
    let request = ChatRequest {
        model: "gpt-4".to_string(),
        messages,
    };

    let response_text = client
        .post(url)
        .headers(headers)
        .json(&request)
        .send()
        .await
        .map_err(|err| anyhow!("request failed: {}", err))?
        .text()
        .await?;

    serde_json::from_str(&response_text).map_err(|err| anyhow!("{}\nBody was: {}", &err, &response_text))
}
#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}
impl ChatMessage {
    pub fn new(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub usage: ChatUsage,
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub message: ChatMessage,
    pub finish_reason: String,
    pub index: u32,
}
