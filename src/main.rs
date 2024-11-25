mod api;
mod config;
mod console;
mod highlight;
mod writer;

use std::io::{self, IsTerminal};

use anyhow::bail;
use futures_util::StreamExt;

use crate::api::{ApiClient, Message, Role};
use crate::config::Config;
use crate::console::Console;
use crate::highlight::highlight_markdown;
use crate::writer::StreamWriter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: config::Args = argh::from_env();
    if args.print_version {
        println!("{} v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let config = config::get_config(&args)?;
    if io::stdin().is_terminal() {
        run_interactive_loop(config).await
    } else {
        run_with_piped_input(config).await
    }
}

fn create_messages(system_prompt: Option<String>) -> Vec<Message> {
    match system_prompt {
        None => Vec::new(),
        Some(prompt) => vec![Message::new(Role::System, &prompt)],
    }
}

async fn get_and_print_completion(
    api_client: &ApiClient,
    messages: &Vec<Message>,
    stream: bool,
) -> anyhow::Result<String> {
    if stream {
        let mut writer = StreamWriter::new();
        let mut events = api_client.stream_chat_completion(messages).await?;
        while let Some(event) = events.next().await {
            if let Some(token) = event? {
                writer.add_token(&token)?;
            }
        }
        let completion = writer.complete()?;
        Ok(completion)
    } else {
        let completion = api_client.get_chat_completion(messages).await?;
        let highlighted = highlight_markdown(&completion)?;
        println!("{}", highlighted);
        Ok(completion)
    }
}

async fn run_interactive_loop(config: Config) -> anyhow::Result<()> {
    let api_client = ApiClient::new(&config);
    let mut console = Console::new()?;
    let mut messages = create_messages(config.system_prompt);
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
            Some(user_prompt) => {
                if user_prompt.is_empty() {
                    continue; // ignore empty lines
                }
                messages.push(Message::new(Role::User, &user_prompt));
            }
        }
        let result = get_and_print_completion(&api_client, &messages, config.stream).await;
        match result {
            Ok(completion) => messages.push(Message::new(Role::Assistant, &completion)),
            Err(error) => eprintln!("[e] {:?}", error),
        }
    }
    Ok(())
}

async fn run_with_piped_input(config: Config) -> anyhow::Result<()> {
    let api_client = ApiClient::new(&config);
    let mut console = Console::new()?;
    let mut messages = create_messages(config.system_prompt);
    let user_prompt = console.read_piped_input()?;
    if user_prompt.is_empty() {
        bail!("Expected a prompt to be supplied via stdin but it was empty");
    }
    messages.push(Message::new(Role::User, &user_prompt));
    get_and_print_completion(&api_client, &messages, config.stream).await?;
    Ok(())
}
