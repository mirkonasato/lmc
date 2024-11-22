mod api;
mod config;
mod console;

use std::io::{self, IsTerminal};

use futures_util::StreamExt;

use crate::api::{ApiClient, ApiError, Message, Role};
use crate::console::Console;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: config::Args = argh::from_env();
    if args.print_version {
        println!("{} v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let config = config::get_config(&args)?;
    let api_client = ApiClient::new(&config);
    let interactive = io::stdin().is_terminal();
    let streaming = !args.no_streaming.unwrap_or(false);

    let mut console = Console::new()?;
    let mut messages: Vec<Message> = Vec::new();
    if let Some(system_prompt) = config.system_prompt {
        messages.push(Message::new(Role::System, &system_prompt));
    }

    if interactive {
        println!(
            "[i] Chatting with \"{}\" at \"{}\"",
            config.model, config.api_url
        );
        loop {
            match console.read_interactive_input()? {
                None => break, // EOF
                Some(command) if command == "/q" || command == "/quit" => break,
                Some(command) if command == "/r" || command == "/retry" => {
                    if let Some(message) = messages.last() {
                        if message.role == Role::Assistant {
                            messages.pop();
                        }
                    }
                }
                Some(user_prompt) => messages.push(Message::new(Role::User, &user_prompt)),
            }
            let result = get_and_print_completion(&api_client, &messages, streaming).await;
            match result {
                Ok(completion) => messages.push(Message::new(Role::Assistant, &completion)),
                Err(error) => eprintln!("[e] {:?}", error),
            }
        }
    } else {
        let user_prompt = console.read_piped_input()?;
        messages.push(Message::new(Role::User, &user_prompt));
        get_and_print_completion(&api_client, &messages, streaming).await?;
    }

    Ok(())
}

async fn get_and_print_completion(
    api_client: &ApiClient,
    messages: &Vec<Message>,
    streaming: bool,
) -> Result<String, ApiError> {
    if !streaming {
        let completion = api_client.get_chat_completion(messages).await?;
        println!("{}", completion);
        Ok(completion)
    } else {
        let mut stream = api_client.stream_chat_completion(messages).await?;
        let mut completion = String::new();
        while let Some(event) = stream.next().await {
            if let Some(token) = event? {
                completion.push_str(&token);
                print!("{}", token);
            }
        }
        println!();
        Ok(completion)
    }
}
