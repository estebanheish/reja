use std::sync::{atomic::AtomicBool, Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;

use crate::config::Profile;

const OLLAMA_CHAT: &str = "http://localhost:11434/api/chat";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    role: String,
    pub content: String,
}

impl Message {
    fn user(s: String) -> Self {
        Self {
            role: "user".to_string(),
            content: s,
        }
    }
    fn assistant(s: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content: s,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Converation {
    pub model: String,
    pub messages: Vec<Message>,
}

impl Converation {
    pub fn from_profile(p: Profile) -> Self {
        let mut messages = vec![];
        if let Some(sys) = p.system {
            messages.push(Message {
                role: "system".to_string(),
                content: sys,
            });
        }
        Self {
            model: p.model,
            messages,
        }
    }
}

#[derive(Deserialize)]
struct Payload {
    message: Message,
}

pub async fn chat(params: Arc<RwLock<Converation>>, new_msg: String, receiving: Arc<AtomicBool>) {
    let client = reqwest::Client::new();

    params.write().await.messages.push(Message::user(new_msg));

    let mut stream = client
        .post(OLLAMA_CHAT)
        .json(&*params.read().await)
        .send()
        .await
        .unwrap()
        .bytes_stream();

    params
        .write()
        .await
        .messages
        .push(Message::assistant("".to_string()));

    tokio::task::spawn(async move {
        receiving.store(true, std::sync::atomic::Ordering::SeqCst);
        while let Some(Ok(chunk)) = stream.next().await {
            if !receiving.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            let pl: Payload = serde_json::from_slice(&chunk).unwrap();
            params
                .write()
                .await
                .messages
                .last_mut()
                .unwrap()
                .content
                .push_str(&pl.message.content);
        }
        receiving.store(false, std::sync::atomic::Ordering::SeqCst);
    });
}
