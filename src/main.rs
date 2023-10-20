mod api;
use api::{stream_response, ChatMessage, ChatRole};

use std::io::{stdin, Read};
use std::str::Lines;
use tokio;
use tokio::io::{stdout, AsyncWriteExt};

#[tokio::main]
async fn main() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("No API key provided");
    let messages = structure_input();
    // println!("{:?}", messages);

    let mut receiver = stream_response(&api_key, messages);

    let mut out = stdout();
    while let Some(token) = receiver.recv().await {
        out.write_all(token.as_bytes()).await.unwrap();
        out.flush().await.unwrap();
    }
    // Append newline to end of output
    println!();
}

fn get_prompt() -> ChatMessage {
    let default_prompt = vec![
        "You are a helpful assistant who provides brief explanations and short code snippets",
        "for linux command-line tools and languages like neovim, Docker, rust and python.",
        "Your user is an expert programmer. You do not show lengthy steps or setup instructions.",
    ]
    .join(" ");
    let prompt = std::env::var("SHELLBOT_PROMPT").unwrap_or(default_prompt);
    return ChatMessage {
        role: ChatRole::System,
        content: prompt,
    };
}

fn structure_input() -> Vec<ChatMessage> {
    let mut input = String::new();
    stdin().read_to_string(&mut input).unwrap();
    let mut lines_iter = input.lines();
    let first_line = lines_iter.next().unwrap();
    match match_separator(first_line) {
        None => vec![
            get_prompt(),
            ChatMessage {
                role: ChatRole::User,
                content: input,
            },
        ],
        Some(first_role) => parse_transcript(first_role, lines_iter),
    }
}

fn parse_transcript(first_role: ChatRole, lines: Lines) -> Vec<ChatMessage> {
    let new_message = |role: ChatRole| ChatMessage {
        role,
        content: String::new(),
    };
    let initial_messages = if first_role == ChatRole::System {
        vec![new_message(first_role)]
    } else {
        vec![get_prompt(), new_message(first_role)]
    };
    lines
        .into_iter()
        .fold(initial_messages, |mut acc: Vec<ChatMessage>, line| {
            match match_separator(line) {
                Some(role) => acc.push(new_message(role)),
                None => {
                    let last = acc.last_mut().unwrap();
                    last.content = format!("{}{}\n", last.content, line)
                }
            }
            acc
        })
}

fn match_separator(line: &str) -> Option<ChatRole> {
    match line {
        "===SYSTEM===" => Some(ChatRole::System),
        "===USER===" => Some(ChatRole::User),
        "===ASSISSTANT===" => Some(ChatRole::Assistant),
        _ => None,
    }
}
