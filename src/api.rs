use bytes::Bytes;
use futures::stream::StreamExt;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, Receiver, Sender};

// const MODEL: &str = "gpt-3.5-turbo";
const MODEL: &str = "gpt-4";
pub fn stream_response<'a>(api_key: &str, messages: Vec<ChatMessage>) -> Receiver<String> {
    let client = get_client(api_key, messages);
    let (sender, receiver) = mpsc::channel(100);
    tokio::spawn(async move { send_response(client, sender).await });
    return receiver;
}

fn get_client(api_key: &str, messages: Vec<ChatMessage>) -> RequestBuilder {
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
    client.post(url).headers(headers).json(&request)
}

async fn send_response(client: RequestBuilder, sender: Sender<String>) {
    let stream = client.send().await.expect("Request failed").bytes_stream();
    let buffer = Arc::new(Mutex::new(String::new()));

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
                    .filter_map(convert_message)
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

fn convert_message(message: String) -> Option<String> {
    // Empty messages are ok
    if message == "" {
        return None;
    }
    // Server-sent events match from beginning of line
    let match_event = Regex::new(r"^(\w+):(.*)$").unwrap();
    let captures = match_event
        .captures(&message)
        .unwrap_or_else(|| panic!("No match for |{}|", message));
    match &captures[1] {
        "data" => match captures[2].trim() {
            "[DONE]" => None,
            event_json => serde_json::from_str::<ChatEvent>(event_json)
                .map(|event| event.choices[0].delta.content.clone())
                .unwrap_or_else(|err| {
                    panic!("Deserialization error {:?} in |{}|", err, &captures[2])
                }),
        },
        event_name => panic!("Unrecognized event {}", event_name),
    }
}

fn process_buffer(input: &String) -> (Vec<String>, String) {
    let mut parts: Vec<String> = input.split("\n\n").map(String::from).collect();
    // let mut parts: Vec<String> = input.split("\n\n").map(|s| s.to_string()).collect();
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
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    User,
    System,
    Assistant,
}
