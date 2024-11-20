use anyhow::{anyhow, Result};
use eventsource_stream::{Event, EventStream, EventStreamError};
use futures_util::{Stream, StreamExt};
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn new(role: Role, content: &String) -> Self {
        Self {
            role,
            content: content.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Assistant,
    System,
    User,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatResponseChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatResponseChoice {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct EventData {
    choices: Vec<ChatEventChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatEventChoice {
    delta: Delta,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

pub struct ApiClient {
    config: Config,
}

impl ApiClient {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self {
            config: config.to_owned(),
        })
    }

    pub async fn get_chat_completion(&self, messages: &Vec<Message>) -> Result<String> {
        let response: ChatResponse = self
            .prepare_request(false, messages)
            .send()
            .await?
            .json()
            .await?;
        match response.choices.first() {
            Some(choice) => Ok(choice.message.content.trim().into()),
            None => Ok("".into()),
        }
    }

    pub async fn stream_chat_completion(
        &self,
        messages: &Vec<Message>,
    ) -> Result<impl Stream<Item = Result<Option<String>>>> {
        let response = self.prepare_request(true, messages).send().await?;
        let stream = EventStream::new(response.bytes_stream()).map(parse_event_data);
        Ok(stream)
    }

    fn prepare_request(&self, stream: bool, messages: &Vec<Message>) -> RequestBuilder {
        let client = Client::new();
        let mut request = client.post(self.config.api_url.clone() + "/chat/completions");
        if stream {
            request = request.header(ACCEPT, "text/event-stream");
        }
        if let Some(key) = &self.config.api_key {
            request = request.bearer_auth(key);
        }
        request
            .header(CONTENT_TYPE, "application/json")
            .json(&ChatRequest {
                model: self.config.model.to_owned(),
                messages: messages.to_owned(),
                stream,
                temperature: self.config.temperature,
            })
    }
}

fn parse_event_data(
    item: Result<Event, EventStreamError<reqwest::Error>>,
) -> Result<Option<String>> {
    match item {
        Ok(event) => {
            if event.data == "[DONE]" {
                return Ok(None);
            }
            let data: EventData = serde_json::from_str(&event.data)?;
            match data.choices.first() {
                Some(choice) => Ok(choice.delta.content.to_owned()),
                None => Ok(None),
            }
        }
        Err(error) => Err(anyhow!("Failed to read event: {}", error)),
    }
}
