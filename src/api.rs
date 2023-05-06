use anyhow::{anyhow, Result};
use futures::{StreamExt, TryStreamExt};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use tokio::io::{stdout, AsyncWriteExt};
use tokio::sync::mpsc::{self, Receiver, Sender};

const MODEL: &str = "gpt-3.5-turbo";
// const MODEL = "gpt-4";
pub async fn make_streamed_request<'a>(
    api_key: &str,
    messages: Vec<ChatMessage>,
) -> Receiver<String> {
    let client = get_client(api_key, messages);
    let (sender, receiver) = mpsc::channel(100);
    tokio::spawn(async move {
        send_response(client, sender).await
    });
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
    let stream = client
        .send()
        .await
        .expect("Request failed")
        .bytes_stream();

    // Server-sent events match from beginning of line
    let match_event = Regex::new(r"^(\w+):(.*)$").unwrap();
    stream
        .for_each(|chunk_result| async {
            let maybe_messages: Result<String> = chunk_result
                .map_err(|err| anyhow!("Stream error {:?}", err))
                .and_then(|chunk| {
                    std::str::from_utf8(&chunk)
                        .map(|s| s.to_owned())
                        .map_err(|err| anyhow!("Encoding error {:?}", err))
                });

            let result: Result<Vec<ChatEvent>> = maybe_messages.and_then(|messages| {
                assert!(
                    messages.ends_with("\n\n"),
                    "Chunks are expected to end with two newline characters."
                );
                messages
                    .split("\n\n")
                    .map(|message| -> Result<ChatEvent> {
                        match_event
                            .captures(message)
                            .ok_or(anyhow!("No match for {}", message))
                            .and_then(|captures| match &captures[1] {
                                "data" => {
                                    serde_json::from_str::<ChatEvent>(&captures[2]).map_err(|err| {
                                        anyhow!("Deserialization error {} in {}", err, &captures[2])
                                    })
                                }
                                event_name => Err(anyhow!("Unrecognized event {}", event_name)),
                            })
                    })
                    .collect()
            });
        })
        .await;
}
    // let mut out = stdout();
// while let Some(chunk_result) = stream.next().await {
//     let chunk_string = std::str::from_utf8(&chunk_result?)?.to_owned();
//     assert!(
//         chunk_string.ends_with("\n\n"),
//         "Chunks are expected to end with two newline characters."
//     );

//     let messages = chunk_string.split("\n\n");
//     let tokens : Vec<String> = messages
//         .filter_map(|line| match_event.captures(line.trim()))
//         .map(|captures| match &captures[1] {
//             "data" => serde_json::from_str(&captures[2])
//                 .map_err(|err| anyhow!("Deserialization error {} in {}", err, &captures[2])),
//             event_name => Err(anyhow!("Unrecognized event {}", event_name)),
//         })
//         .collect::<Result<Vec<String>>>()?;

//     for token in tokens {
//         out.write_all(token.as_bytes()).await?;
//     }
//     out.flush().await?;
// }

// Ok(())
// }

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
