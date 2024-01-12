use bytes::Bytes;
use futures::stream::StreamExt;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::anthropic;
use crate::openai;
use crate::sse::SSEvent;
use crate::sse::SseConverter;

pub enum ApiProvider {
    OpenAI(String),
    Anthropic(String),
}

#[derive(Clone, Copy)]
enum ProviderType {
    OpenAI,
    Anthropic,
}

impl ApiProvider {
    fn get_type(&self) -> ProviderType {
        match self {
            ApiProvider::OpenAI(_) => ProviderType::OpenAI,
            ApiProvider::Anthropic(_) => ProviderType::Anthropic,
        }
    }
}

pub fn stream_response<'a>(provider: ApiProvider, request: ChatRequest) -> Receiver<String> {
    let provider_type: ProviderType = provider.get_type();
    let request = match provider {
        ApiProvider::OpenAI(api_key) => openai::get_request(&api_key, request),
        ApiProvider::Anthropic(api_key) => anthropic::get_request(&api_key, request),
    };
    let (sender, receiver) = mpsc::channel(100);
    tokio::spawn(async move { send_response(provider_type, request, sender).await });
    return receiver;
}

async fn send_response(provider: ProviderType, client: RequestBuilder, sender: Sender<String>) {
    let stream = client.send().await.expect("Request failed").bytes_stream();
    let buffer = Arc::new(Mutex::new(String::new()));
    let sse_converter = &SseConverter::new();

    stream
        .map(|chunk_result| {
            let buffer = Arc::clone(&buffer);
            async move {
                let result = chunk_result.expect("Stream error");
                let mut locked_buffer = buffer.lock().unwrap();
                locked_buffer.push_str(&convert_chunk(result));
                let (m, rest) = process_buffer(&locked_buffer);
                locked_buffer.clear();
                locked_buffer.push_str(&rest);
                m.into_iter()
                    .filter_map(|string_sse| sse_converter.convert(string_sse))
                    .filter_map(|sse| process_sse(provider, sse))
                    .collect::<Vec<_>>()
            }
        })
        .for_each(|tokens| async {
            for token in tokens.await {
                sender
                    .send(token)
                    .await
                    .unwrap_or_else(|_| panic!("Failed to send token"));
            }
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

fn process_sse(provider: ProviderType, event: SSEvent) -> Option<String> {
    match provider {
        ProviderType::Anthropic => anthropic::convert_sse(event),
        ProviderType::OpenAI => openai::convert_sse(event),
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
