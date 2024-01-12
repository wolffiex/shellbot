mod anthropic;
mod api;
mod openai;
mod sse;

use api::{stream_response, ApiProvider, ChatMessage, ChatRequest, ChatRole};

use std::io::{stdin, Read};
use std::str::Lines;
use tokio;
use tokio::io::{stdout, AsyncWriteExt};

#[tokio::main]
async fn main() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("No API key provided");
    let request = structure_input();
    // println!("{:?}", messages);

    let mut receiver = stream_response(ApiProvider::OpenAI(api_key), request);

    let mut out = stdout();
    while let Some(token) = receiver.recv().await {
        out.write_all(token.as_bytes()).await.unwrap();
        out.flush().await.unwrap();
    }
    // Append newline to end of output
    println!();
}

fn get_default_prompt() -> String {
    vec![
        "You are a helpful assistant who provides brief explanations and short code snippets",
        "for linux command-line tools and languages like neovim, Docker, rust and python.",
        "Your user is an expert programmer. You do not show lengthy steps or setup instructions.",
        "Only provide answers in cases where you know the answer. Feel free to say \"I don't know.\"",
        "Do not suggest contacting support."
    ].join(" ")
}

fn structure_input() -> ChatRequest {
    let mut input = String::new();
    stdin().read_to_string(&mut input).unwrap();
    let mut lines_iter = input.lines();
    let first_line = lines_iter.next().unwrap();
    match match_separator(first_line) {
        None => ChatRequest {
            system_prompt: get_default_prompt(),
            transcript: vec![ChatMessage {
                role: ChatRole::User,
                content: input,
            }],
        },
        Some(first_role) => parse_transcript(first_role, lines_iter),
    }
}

fn parse_transcript(first_role: ChatRole, lines: Lines) -> ChatRequest {
    let new_message = |role: ChatRole| ChatMessage {
        role,
        content: String::new(),
    };
    let mut transcript = lines.into_iter().fold(
        vec![new_message(first_role)],
        |mut acc: Vec<ChatMessage>, line| {
            match match_separator(line) {
                Some(role) => acc.push(new_message(role)),
                None => {
                    let last = acc.last_mut().unwrap();
                    last.content = format!("{}{}\n", last.content, line)
                }
            }
            acc
        },
    );

    let system_prompt = if transcript.get(0).unwrap().role == ChatRole::System {
        transcript.remove(0).content
    } else {
        get_default_prompt()
    };
    ChatRequest {
        system_prompt,
        transcript,
    }
}

fn match_separator(line: &str) -> Option<ChatRole> {
    match line {
        "===SYSTEM===" => Some(ChatRole::System),
        "===USER===" => Some(ChatRole::User),
        "===ASSISSTANT===" => Some(ChatRole::Assistant),
        _ => None,
    }
}
