mod api;
use api::{stream_response, ChatMessage};
use std::io::{self, Read};
use tokio::{self, io::stdout, io::AsyncWriteExt};

#[tokio::main]
async fn main() {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).unwrap();

    let api_key = std::env::var("OPENAI_API_KEY")
        .ok()
        .expect("No API key provided");
    let messages = vec![get_system_prompt(), ChatMessage::new("user", &input)];

    let mut receiver = stream_response(&api_key, messages);

    let mut out = stdout();
    while let Some(token) = receiver.recv().await {
        out.write_all(token.as_bytes()).await.unwrap();
        out.flush().await.unwrap();
    }
}

fn get_system_prompt() -> ChatMessage {
    let prompt_lines = vec![
        "You are a helpful assistant who provides very brief explanations and short code snippets.",
        "You do not show steps or setup instructions.",
        "When the user provides an error message, try to explain it.",
    ];
    return ChatMessage {
        role: "system".to_string(),
        content: prompt_lines.join(" "),
    };
}
