use anyhow::{anyhow, Result};
use futures::{StreamExt, TryStreamExt};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::io::{stdout, AsyncWriteExt};

const MODEL: &str = "gpt-3.5-turbo";
// const MODEL = "gpt-4";
pub async fn make_streamed_request<'a>(api_key: &str, messages: Vec<ChatMessage>) -> Result<()> {
    let client = Client::new();
    let url = "https://api.openai.com/v1/chat/completions";
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("text/event-stream"),
    );
    headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {}", api_key)).unwrap(),
    );
    let request = ChatRequest {
        model: MODEL.to_string(),
        stream: true,
        messages,
    };

    let mut stream = client
        .post(url)
        .headers(headers)
        .json(&request)
        .send()
        .await
        .map_err(|err| anyhow!("request failed: {}", err))?
        .bytes_stream()
        .map_err(|err| anyhow!("stream error: {}", err));

    let mut buffer = String::new();

    let data_event = "data: ";
    let mut out = stdout();
    while let Some(chunk_result) = stream.next().await {
        buffer.push_str(std::str::from_utf8(&chunk_result?)?);
        if let Some(pos) = buffer.find(data_event) {
            buffer = buffer[pos + data_event.len()..].to_owned();
            if let Some(end) = buffer.find("\n\n") {
                let message = buffer[..end].trim().to_owned();
                let my_data: ChatEvent = serde_json::from_str(&message).unwrap();
                if let Some(content) = &my_data.choices[0].delta.content {
                    out.write_all(content.to_string().as_bytes()).await?;
                    out.flush().await?;
                }
                buffer = buffer[end + 2..].to_owned();
            }
        }
    }

    Ok(())
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
    stream: bool,
    messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChatEvent {
    id: String,
    object: String,
    created: i64,
    model: String,
    pub choices: Vec<Choice>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Choice {
    pub delta: Delta,
    index: i32,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Delta {
    pub content: Option<String>,
}
