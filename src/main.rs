mod api;
use api::{make_streamed_request, ChatMessage};
use std::io::{self, Read};
use tokio;

#[tokio::main]
async fn main() {
    let mut input = String::new();
    println!("ready?");
    io::stdin().read_to_string(&mut input).unwrap();

    let api_key = std::env::var("OPENAI_API_KEY")
        .ok()
        .expect("No API key provided");
    let messages = vec![get_system_prompt(), ChatMessage::new("user", &input)];

    match make_streamed_request(&api_key, messages).await {
        Ok(()) => (),
        Err(err) => eprintln!("Error: {}", err),
    };
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
