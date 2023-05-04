mod api;
use api::{make_request, ChatMessage};
use tokio;


#[tokio::main]
async fn main() {
    let api_key = std::env::var("OPENAI_API_KEY").ok().expect("No API key provided");
    let prompt = "Write a Rust function that takes a string and reverses it.";
    let messages = vec![ChatMessage::new("user" ,&prompt )];

    match make_request(&api_key, messages).await {
        Ok(response) => {
            println!("Result:");
            println!("  id: {}", response.id);
            println!("  object: {}", response.object);
            println!("  created: {}", response.created);
            println!("  model: {}", response.model);
            println!("  usage:");
            println!("    prompt_tokens: {}", response.usage.prompt_tokens);
            println!("    completion_tokens: {}", response.usage.completion_tokens);
            println!("    total_tokens: {}", response.usage.total_tokens);
            println!("  choices:");
            for choice in &response.choices {
                println!("  - message:");
                println!("      role: {}", choice.message.role);
                println!("      content: {}", choice.message.content);
                println!("    finish_reason: {}", choice.finish_reason);
                println!("    index: {}", choice.index);
            }
        }
        Err(err) => eprintln!("Error: {}", err),
    }
}

