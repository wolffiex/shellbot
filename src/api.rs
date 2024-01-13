use bytes::Bytes;
use futures::stream::StreamExt;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::anthropic;
use crate::openai;
use crate::sse::SSEvent;
use crate::sse::SSEConverter;

pub enum ApiProvider {
    OpenAI(String),
    Anthropic(String),
}

pub fn stream_response<'a>(provider: ApiProvider, request: ChatRequest) -> Receiver<String> {
    let request = match provider {
        ApiProvider::OpenAI(ref api_key) => openai::get_request(&api_key, request),
        ApiProvider::Anthropic(ref api_key) => anthropic::get_request(&api_key, request),
    };
    let (sender, receiver) = mpsc::channel(100);
    tokio::spawn(async move { send_response(&provider, request, sender).await });
    return receiver;
}

async fn send_response(provider: &ApiProvider, client: RequestBuilder, sender: Sender<String>) {
    let stream = client.send().await.expect("Request failed").bytes_stream();
    let mut buffer = String::new();
    let sse_converter = &SSEConverter::new();

    stream
        .map(|chunk_result| {
            let result = chunk_result.expect("Stream error");
            buffer.push_str(&convert_chunk(result));
            let (m, rest) = process_buffer(&buffer);
            buffer = rest.to_string();
            m.into_iter()
                .filter_map(|string_sse| sse_converter.convert(string_sse))
                .filter_map(|sse| process_sse(&provider, sse))
                .collect::<Vec<String>>()
                .join("")
        })
        .for_each(|str| async {
            sender.send(str).await.expect("Failed to send token");
        })
        .await;
}

fn process_buffer(input: &String) -> (Vec<String>, String) {
    let mut parts: Vec<String> = input.split("\n\n").map(String::from).collect();
    let remainder = if input.ends_with("\n\n") {
        None
    } else {
        parts.pop()
    };
    (parts, remainder.unwrap_or(String::new()))
}

fn convert_chunk(chunk: Bytes) -> String {
    std::str::from_utf8(&chunk)
        .map(String::from)
        .expect("Encoding error")
}

fn process_sse(provider: &ApiProvider, event: SSEvent) -> Option<String> {
    match provider {
        ApiProvider::Anthropic(_) => anthropic::convert_sse(event),
        ApiProvider::OpenAI(_) => openai::convert_sse(event),
    }
}

pub struct ChatRequest {
    pub system_prompt: String,
    pub transcript: Vec<ChatMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    User,
    System,
    Assistant,
}
