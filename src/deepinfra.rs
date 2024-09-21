use std::collections::HashMap;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use log::info;

static mut DEEPINFRA_CHAT: Option<DeepInfraChatInner> = None;

pub struct DeepInfraChat;

impl DeepInfraChat {
    pub fn init(model: &str, temperature: f32, max_tokens: i32) {
        unsafe {
            DEEPINFRA_CHAT = Some(DeepInfraChatInner::new(model, temperature, max_tokens));
        }
    }

    pub fn global() -> &'static mut DeepInfraChatInner {
        unsafe {
            DEEPINFRA_CHAT.as_mut().unwrap()
        }
    }
}

#[derive(Clone)]
pub struct DeepInfraChatInner {
    model: String,
    temperature: f32,
    max_tokens: i32,
    contexts: HashMap<i64, DeepInfraContext>
}

impl DeepInfraChatInner {
    pub fn new(model: &str, temperature: f32, max_tokens: i32) -> Self {
        Self {
            model: model.to_string(),
            temperature,
            max_tokens,
            contexts: Default::default(),
        }
    }

    pub fn get(&mut self, id: i64) -> &mut DeepInfraContext {
        if !self.contexts.contains_key(&id) {
            self.contexts.insert(id, DeepInfraContext::new(self.model.clone(), false, self.temperature, self.max_tokens));
        }
        self.contexts.get_mut(&id).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DeepInfraChatContent {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub content: String,
}

impl DeepInfraChatContent {
    pub fn new(role: &str, content: &str) -> Self {
        DeepInfraChatContent {
            role: role.to_string(),
            name: None,
            content: content.to_string(),
        }
    }

    pub fn new_named(role: &str, name: &str, content: &str) -> Self {
        DeepInfraChatContent {
            role: role.to_string(),
            name: Some(name.to_string()),
            content: content.to_string(),
        }
    }
}

#[derive(Clone)]
pub struct DeepInfraContext {
    history: Vec<DeepInfraChatContent>,
    model: String,
    stream: bool,
    temperature: f32,
    max_tokens: i32,
}

impl DeepInfraContext {
    pub fn new(model: String, stream: bool, temperature: f32, max_tokens: i32) -> Self {
        DeepInfraContext {
            history: vec![],
            model,
            stream,
            temperature,
            max_tokens,
        }
    }

    pub async fn clear(&mut self) {
        self.history.clear();
    }

    pub async fn recall(&mut self) {
        let last = self.history.pop();
        if let Some(last) = last {
            if last.role == "assistant" {
                self.history.pop();
            }
        }
    }

    pub async fn chat(&mut self, prompt: String, name: Option<String>, message: &str) -> anyhow::Result<String> {
        if let Some(name) = name {
            self.history.push(DeepInfraChatContent::new_named("user", &name, message));
        }else {
            self.history.push(DeepInfraChatContent::new("user", message));
        }

        let mut vec = self.history.clone();
        vec.insert(0, DeepInfraChatContent::new("system", &prompt));
        let messages = serde_json::to_value(vec).unwrap();

        info!("对话JSON >> {}", serde_json::to_string(&messages).unwrap());

        let message = self.chat0(messages).await?;
        self.history.push(DeepInfraChatContent::new("assistant", &message));
        Ok(message)
    }

    async fn chat0(&self, messages: Value) -> anyhow::Result<String> {
        let mut headers = HeaderMap::new();
        headers.insert("Accept-Encoding", "gzip, deflate, br".parse().unwrap());
        headers.insert("Accept-Language", "en-US".parse().unwrap());
        headers.insert("Connection", "keep-alive".parse().unwrap());
        headers.insert("Content-Type", "application/json".parse().unwrap());
        headers.insert("Origin", "https://deepinfra.com".parse().unwrap());
        headers.insert("Referer", "https://deepinfra.com/".parse().unwrap());
        headers.insert("Sec-Fetch-Dest", "empty".parse().unwrap());
        headers.insert("Sec-Fetch-Mode", "cors".parse().unwrap());
        headers.insert("Sec-Fetch-Site", "same-site".parse().unwrap());
        headers.insert("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36".parse().unwrap());
        headers.insert("X-Deepinfra-Source", "web-embed".parse().unwrap());
        headers.insert("accept", "text/event-stream".parse().unwrap());
        headers.insert("sec-ch-ua", "\"Google Chrome\";v=\"119\", \"Chromium\";v=\"119\", \"Not?A_Brand\";v=\"24\"".parse().unwrap());
        headers.insert("sec-ch-ua-mobile", "?0".parse().unwrap());
        headers.insert("sec-ch-ua-platform", "\"macOS\"".parse().unwrap());

        let resp = reqwest::Client::new()
            .post("https://api.deepinfra.com/v1/openai/chat/completions")
            .headers(headers)
            .json(&json!({
                "model": self.model,
                "messages": messages,
                "stream": self.stream,
                "temperature": self.temperature,
                "max_tokens": self.max_tokens
            }))
            .send().await?;
        if resp.status().is_success() {
            let json: Value = resp.json().await?;
            let choices = json["choices"].as_array().unwrap();
            let message = choices[0]["message"]["content"].as_str().unwrap();
            Ok(message.to_string())
        } else {
            Err(anyhow::anyhow!("{}", resp.text().await?))
        }
    }
}