use crate::api::ChatRequest;
use crate::sse::SSEvent;
use crate::ChatMessage;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};

const MODEL: &str = "claude-2.1";
pub fn get_request(api_key: &str, request: ChatRequest) -> RequestBuilder {
    let client = Client::new();
    let url = "https://api.anthropic.com/v1/messages";
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("text/event-stream"),
    );
    headers.insert(
        "X-Api-Key",
        HeaderValue::from_str(&format!("{}", api_key)).unwrap(),
    );
    headers.insert(
        "anthropic-version",
        HeaderValue::from_str("2023-06-01").unwrap(),
    );
    headers.insert(
        "Anthropic-Beta",
        HeaderValue::from_str("messages-2023-12-15").unwrap(),
    );

    let request = RequestJSON {
        model: MODEL.to_string(),
        system: request.system_prompt,
        messages: request.transcript,
        stream: true,
        max_tokens: 512,
    };
    client.post(url).headers(headers).json(&request)
}

fn convert_sse(event: SSEvent) -> Option<String> {
    match event.name {
        Some(name) if name == "content_block_delta" => {
            let parsed_data = serde_json::from_str::<ContentBlockDelta>(&event.data);
            Some(parsed_data.unwrap().delta.text)
        }
        _ => None,
    }
}

#[derive(Debug, Serialize)]
struct RequestJSON {
    model: String,
    stream: bool,
    messages: Vec<ChatMessage>,
    system: String,
    max_tokens: usize,
}

#[derive(Deserialize, Debug)]
struct ContentBlockDelta {
    delta: Delta,
}

#[derive(Deserialize, Debug)]
struct Delta {
    text: String,
}