use bytes::Bytes;
use futures::stream::StreamExt;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, Receiver, Sender};

use crate::openai;
use crate::sse::SseConverter;

// const MODEL: &str = "gpt-3.5-turbo";
pub fn stream_response<'a>(api_key: &str, messages: Vec<ChatMessage>) -> Receiver<String> {
    let client = openai::get_request(api_key, messages);
    let (sender, receiver) = mpsc::channel(100);
    tokio::spawn(async move { send_response(client, sender).await });
    return receiver;
}

async fn send_response(client: RequestBuilder, sender: Sender<String>) {
    let stream = client.send().await.expect("Request failed").bytes_stream();
    let buffer = Arc::new(Mutex::new(String::new()));
    let sse_re = &SseConverter::new();

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
                    .filter_map(|string_sse| sse_re.convert_sse(string_sse))
                    .filter_map(openai::convert_sse)
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    User,
    System,
    Assistant,
}
