use crate::core::plugin::{Context, ExecutionResult, Plugin, SearchResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

pub struct AiPlugin {
    provider: String,
    api_key: String,
    model: String,
    api_url: String,
    last_response: Arc<RwLock<Option<String>>>,
    loading: Arc<RwLock<bool>>,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessageResponse,
}

#[derive(Deserialize)]
struct OpenAIMessageResponse {
    content: String,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

impl AiPlugin {
    pub fn new(provider: String, api_key: String, model: String, api_url: String) -> Self {
        Self {
            provider,
            api_key,
            model,
            api_url,
            last_response: Arc::new(RwLock::new(None)),
            loading: Arc::new(RwLock::new(false)),
        }
    }

    async fn fetch_ai_response(
        provider: &str,
        api_key: &str,
        model: &str,
        api_url: &str,
        query: &str,
    ) -> Result<String, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Client builder error: {e}"))?;

        if provider.to_lowercase() == "ollama" {
            let url = if api_url.is_empty() {
                "http://localhost:11434/api/generate"
            } else {
                api_url
            };

            let req_body = OllamaRequest {
                model: model.to_string(),
                prompt: query.to_string(),
                stream: false,
            };

            let res = client
                .post(url)
                .json(&req_body)
                .send()
                .await
                .map_err(|e| format!("Request failed: {e}"))?;

            if !res.status().is_success() {
                return Err(format!("Ollama API returned status: {}", res.status()));
            }

            let body: OllamaResponse = res
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {e}"))?;

            Ok(body.response)
        } else {
            // OpenAI or Gemini (using OpenAI compatibility endpoint)
            let default_url = if provider.to_lowercase() == "gemini" {
                "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
            } else {
                "https://api.openai.com/v1/chat/completions"
            };

            let url = if api_url.is_empty() {
                default_url
            } else {
                api_url
            };

            let req_body = OpenAIRequest {
                model: model.to_string(),
                messages: vec![OpenAIMessage {
                    role: "user".to_string(),
                    content: query.to_string(),
                }],
            };

            let mut request = client.post(url).json(&req_body);
            if !api_key.is_empty() {
                request = request.header("Authorization", format!("Bearer {api_key}"));
            }

            let res = request
                .send()
                .await
                .map_err(|e| format!("Request failed: {e}"))?;

            if !res.status().is_success() {
                let status = res.status();
                let text = res.text().await.unwrap_or_default();
                return Err(format!("API returned status {status}: {text}"));
            }

            let body: OpenAIResponse = res
                .json()
                .await
                .map_err(|e| format!("Failed to parse response: {e}"))?;

            if let Some(choice) = body.choices.first() {
                Ok(choice.message.content.clone())
            } else {
                Err("Empty choice list returned from API".to_string())
            }
        }
    }
}

impl Plugin for AiPlugin {
    fn id(&self) -> &'static str {
        "ai"
    }

    fn name(&self) -> &'static str {
        "AI Assistant"
    }

    fn description(&self) -> &'static str {
        "Ask questions to an AI model"
    }

    fn search(&self, query: &str, _cache_dir: &Path) -> Vec<SearchResult> {
        if query.trim().is_empty() {
            return Vec::new();
        }

        let mut metadata = HashMap::new();
        metadata.insert("prompt".to_string(), query.to_string());

        vec![SearchResult {
            id: "ai_query".to_string(),
            title: format!("Ask AI: \"{}\"", query),
            subtitle: Some(format!(
                "Provider: {} | Model: {}",
                self.provider, self.model
            )),
            score: 1000,
            plugin_id: self.id(),
            metadata,
        }]
    }

    fn preview(&self, _item: &SearchResult) -> Option<String> {
        let is_loading = *self.loading.read().unwrap();
        let response = self.last_response.read().unwrap().clone();

        if is_loading {
            Some("# AI Assistant\n\n*Thinking... Please wait.*".to_string())
        } else if let Some(resp) = response {
            Some(format!("# AI Assistant Response\n\n{resp}"))
        } else {
            Some(format!(
                "# AI Assistant\n\nConfigured Provider: `{}`\nModel: `{}`\n\n*Press Enter to send query.*",
                self.provider, self.model
            ))
        }
    }

    fn execute(&self, item: &SearchResult, _ctx: &mut Context) -> ExecutionResult {
        let prompt = match item.metadata.get("prompt") {
            Some(p) => p.clone(),
            None => return ExecutionResult::Message("No prompt provided".to_string()),
        };

        // Set state to loading
        if let Ok(mut l) = self.loading.write() {
            *l = true;
        }

        let provider = self.provider.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();
        let api_url = self.api_url.clone();
        let response_store = self.last_response.clone();
        let loading_store = self.loading.clone();

        // Spawn async worker
        tokio::spawn(async move {
            let res = Self::fetch_ai_response(&provider, &api_key, &model, &api_url, &prompt).await;

            if let Ok(mut r) = response_store.write() {
                match res {
                    Ok(text) => *r = Some(text),
                    Err(e) => *r = Some(format!("**Error calling API**:\n`{e}`")),
                }
            }

            if let Ok(mut l) = loading_store.write() {
                *l = false;
            }
        });

        ExecutionResult::Success // Keep launcher open so response can load
    }
}
