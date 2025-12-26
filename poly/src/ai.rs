//! AI/LLM Integration for Poly Applications
//! 
//! Supports:
//! - Ollama (local)
//! - OpenAI API
//! - OpenAI-compatible APIs (LM Studio, LocalAI, etc.)
//! - Streaming with thinking/reasoning support

use serde::{Deserialize, Serialize};

/// AI Provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    Ollama,
    OpenAI,
    Anthropic,
    Custom,
}

impl Default for AiProvider {
    fn default() -> Self {
        AiProvider::Ollama
    }
}

/// Chat message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

/// Content block types (for reasoning/thinking support)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Thinking { thinking: String },
}

/// Chat request configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    #[serde(default)]
    pub provider: AiProvider,
    
    /// Base URL (auto-detected for known providers)
    pub base_url: Option<String>,
    
    /// API key (required for OpenAI, Anthropic)
    pub api_key: Option<String>,
    
    /// Model name
    pub model: String,
    
    /// Chat messages
    pub messages: Vec<ChatMessage>,
    
    /// Temperature (0.0 - 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    
    /// Max tokens to generate
    pub max_tokens: Option<u32>,
    
    /// Enable streaming
    #[serde(default)]
    pub stream: bool,
    
    /// Enable extended thinking (for supported models)
    #[serde(default)]
    pub enable_thinking: bool,
    
    /// Thinking budget tokens (for Anthropic)
    pub thinking_budget: Option<u32>,
}

fn default_temperature() -> f32 { 0.7 }

/// Chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The generated text content
    pub content: String,
    
    /// Thinking/reasoning content (if available)
    pub thinking: Option<String>,
    
    /// Model used
    pub model: String,
    
    /// Token usage
    pub usage: Option<TokenUsage>,
    
    /// Whether response was streamed
    pub streamed: bool,
    
    /// Provider used
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

/// Stream event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Regular content chunk
    Content { delta: String },
    /// Thinking/reasoning chunk
    Thinking { delta: String },
    /// Stream completed
    Done { 
        content: String, 
        thinking: Option<String>,
        usage: Option<TokenUsage>,
    },
    /// Error occurred
    Error { message: String },
}

/// Get base URL for provider
fn get_base_url(provider: &AiProvider, custom_url: Option<&str>) -> String {
    match custom_url {
        Some(url) => url.to_string(),
        None => match provider {
            AiProvider::Ollama => "http://localhost:11434".to_string(),
            AiProvider::OpenAI => "https://api.openai.com".to_string(),
            AiProvider::Anthropic => "https://api.anthropic.com".to_string(),
            AiProvider::Custom => "http://localhost:8080".to_string(),
        }
    }
}

/// Check if Ollama is running
#[cfg(feature = "native")]
pub fn check_ollama() -> Result<bool, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .map_err(|e| e.to_string())?;
    
    match client.get("http://localhost:11434/api/tags").send() {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(_) => Ok(false),
    }
}

/// List available Ollama models
#[cfg(feature = "native")]
pub fn list_ollama_models() -> Result<Vec<String>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;
    
    let resp = client.get("http://localhost:11434/api/tags")
        .send()
        .map_err(|e| format!("Failed to connect to Ollama: {}", e))?;
    
    if !resp.status().is_success() {
        return Err("Ollama not available".to_string());
    }
    
    #[derive(Deserialize)]
    struct OllamaModels {
        models: Vec<OllamaModel>,
    }
    
    #[derive(Deserialize)]
    struct OllamaModel {
        name: String,
    }
    
    let models: OllamaModels = resp.json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    Ok(models.models.into_iter().map(|m| m.name).collect())
}

/// Send chat request (non-streaming)
#[cfg(feature = "native")]
pub fn chat(request: &ChatRequest) -> Result<ChatResponse, String> {
    let base_url = get_base_url(&request.provider, request.base_url.as_deref());
    
    match request.provider {
        AiProvider::Ollama => chat_ollama(&base_url, request),
        AiProvider::OpenAI | AiProvider::Custom => chat_openai(&base_url, request),
        AiProvider::Anthropic => chat_anthropic(&base_url, request),
    }
}

/// Ollama chat implementation
#[cfg(feature = "native")]
fn chat_ollama(base_url: &str, request: &ChatRequest) -> Result<ChatResponse, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    
    #[derive(Serialize)]
    struct OllamaRequest {
        model: String,
        messages: Vec<OllamaMessage>,
        stream: bool,
        options: OllamaOptions,
    }
    
    #[derive(Serialize)]
    struct OllamaMessage {
        role: String,
        content: String,
    }
    
    #[derive(Serialize)]
    struct OllamaOptions {
        temperature: f32,
        #[serde(skip_serializing_if = "Option::is_none")]
        num_predict: Option<u32>,
    }
    
    let messages: Vec<OllamaMessage> = request.messages.iter().map(|m| {
        OllamaMessage {
            role: match m.role {
                MessageRole::System => "system".to_string(),
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
            },
            content: m.content.clone(),
        }
    }).collect();
    
    let ollama_req = OllamaRequest {
        model: request.model.clone(),
        messages,
        stream: false,
        options: OllamaOptions {
            temperature: request.temperature,
            num_predict: request.max_tokens,
        },
    };
    
    let resp = client.post(format!("{}/api/chat", base_url))
        .json(&ollama_req)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if !resp.status().is_success() {
        return Err(format!("Ollama error: {}", resp.status()));
    }
    
    #[derive(Deserialize)]
    struct OllamaResponse {
        message: OllamaResponseMessage,
        #[serde(default)]
        eval_count: Option<u32>,
        #[serde(default)]
        prompt_eval_count: Option<u32>,
    }
    
    #[derive(Deserialize)]
    struct OllamaResponseMessage {
        content: String,
    }
    
    let ollama_resp: OllamaResponse = resp.json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    // Check for thinking tags in content (some models use <think> tags)
    let (content, thinking) = extract_thinking(&ollama_resp.message.content);
    
    Ok(ChatResponse {
        content,
        thinking,
        model: request.model.clone(),
        usage: Some(TokenUsage {
            prompt_tokens: ollama_resp.prompt_eval_count,
            completion_tokens: ollama_resp.eval_count,
            total_tokens: None,
        }),
        streamed: false,
        provider: "ollama".to_string(),
    })
}

/// OpenAI-compatible chat implementation
#[cfg(feature = "native")]
fn chat_openai(base_url: &str, request: &ChatRequest) -> Result<ChatResponse, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    
    #[derive(Serialize)]
    struct OpenAIRequest {
        model: String,
        messages: Vec<OpenAIMessage>,
        temperature: f32,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_tokens: Option<u32>,
        stream: bool,
    }
    
    #[derive(Serialize)]
    struct OpenAIMessage {
        role: String,
        content: String,
    }
    
    let messages: Vec<OpenAIMessage> = request.messages.iter().map(|m| {
        OpenAIMessage {
            role: match m.role {
                MessageRole::System => "system".to_string(),
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
            },
            content: m.content.clone(),
        }
    }).collect();
    
    let openai_req = OpenAIRequest {
        model: request.model.clone(),
        messages,
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        stream: false,
    };
    
    let mut req_builder = client.post(format!("{}/v1/chat/completions", base_url))
        .json(&openai_req);
    
    if let Some(ref api_key) = request.api_key {
        req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
    }
    
    let resp = req_builder.send()
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("API error {}: {}", status, body));
    }
    
    #[derive(Deserialize)]
    struct OpenAIResponse {
        choices: Vec<OpenAIChoice>,
        usage: Option<OpenAIUsage>,
    }
    
    #[derive(Deserialize)]
    struct OpenAIChoice {
        message: OpenAIResponseMessage,
    }
    
    #[derive(Deserialize)]
    struct OpenAIResponseMessage {
        content: Option<String>,
    }
    
    #[derive(Deserialize)]
    struct OpenAIUsage {
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
    }
    
    let openai_resp: OpenAIResponse = resp.json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    let content = openai_resp.choices.first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();
    
    let (content, thinking) = extract_thinking(&content);
    
    Ok(ChatResponse {
        content,
        thinking,
        model: request.model.clone(),
        usage: openai_resp.usage.map(|u| TokenUsage {
            prompt_tokens: Some(u.prompt_tokens),
            completion_tokens: Some(u.completion_tokens),
            total_tokens: Some(u.total_tokens),
        }),
        streamed: false,
        provider: "openai".to_string(),
    })
}

/// Anthropic chat implementation with extended thinking support
#[cfg(feature = "native")]
fn chat_anthropic(base_url: &str, request: &ChatRequest) -> Result<ChatResponse, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    
    let api_key = request.api_key.as_ref()
        .ok_or("API key required for Anthropic")?;
    
    #[derive(Serialize)]
    struct AnthropicRequest {
        model: String,
        messages: Vec<AnthropicMessage>,
        max_tokens: u32,
        #[serde(skip_serializing_if = "Option::is_none")]
        system: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking: Option<ThinkingConfig>,
    }
    
    #[derive(Serialize)]
    struct ThinkingConfig {
        #[serde(rename = "type")]
        thinking_type: String,
        budget_tokens: u32,
    }
    
    #[derive(Serialize)]
    struct AnthropicMessage {
        role: String,
        content: String,
    }
    
    // Extract system message
    let system_msg = request.messages.iter()
        .find(|m| matches!(m.role, MessageRole::System))
        .map(|m| m.content.clone());
    
    let messages: Vec<AnthropicMessage> = request.messages.iter()
        .filter(|m| !matches!(m.role, MessageRole::System))
        .map(|m| {
            AnthropicMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "user".to_string(), // shouldn't happen
                },
                content: m.content.clone(),
            }
        }).collect();
    
    let thinking_config = if request.enable_thinking {
        Some(ThinkingConfig {
            thinking_type: "enabled".to_string(),
            budget_tokens: request.thinking_budget.unwrap_or(10000),
        })
    } else {
        None
    };
    
    let anthropic_req = AnthropicRequest {
        model: request.model.clone(),
        messages,
        max_tokens: request.max_tokens.unwrap_or(4096),
        system: system_msg,
        thinking: thinking_config,
    };
    
    let resp = client.post(format!("{}/v1/messages", base_url))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&anthropic_req)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(format!("Anthropic error {}: {}", status, body));
    }
    
    #[derive(Deserialize)]
    struct AnthropicResponse {
        content: Vec<AnthropicContent>,
        usage: Option<AnthropicUsage>,
    }
    
    #[derive(Deserialize)]
    #[serde(tag = "type")]
    enum AnthropicContent {
        #[serde(rename = "text")]
        Text { text: String },
        #[serde(rename = "thinking")]
        Thinking { thinking: String },
    }
    
    #[derive(Deserialize)]
    struct AnthropicUsage {
        input_tokens: u32,
        output_tokens: u32,
    }
    
    let anthropic_resp: AnthropicResponse = resp.json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    let mut content = String::new();
    let mut thinking = None;
    
    for block in anthropic_resp.content {
        match block {
            AnthropicContent::Text { text } => content.push_str(&text),
            AnthropicContent::Thinking { thinking: t } => thinking = Some(t),
        }
    }
    
    Ok(ChatResponse {
        content,
        thinking,
        model: request.model.clone(),
        usage: anthropic_resp.usage.map(|u| TokenUsage {
            prompt_tokens: Some(u.input_tokens),
            completion_tokens: Some(u.output_tokens),
            total_tokens: Some(u.input_tokens + u.output_tokens),
        }),
        streamed: false,
        provider: "anthropic".to_string(),
    })
}

/// Extract thinking content from <think> tags (for models that use them)
fn extract_thinking(content: &str) -> (String, Option<String>) {
    // Check for <think>...</think> tags (DeepSeek, some Ollama models)
    if let Some(start) = content.find("<think>") {
        if let Some(end) = content.find("</think>") {
            let thinking = content[start + 7..end].trim().to_string();
            let before = &content[..start];
            let after = &content[end + 8..];
            let clean_content = format!("{}{}", before, after).trim().to_string();
            return (clean_content, Some(thinking));
        }
    }
    
    // Check for <thinking>...</thinking> tags
    if let Some(start) = content.find("<thinking>") {
        if let Some(end) = content.find("</thinking>") {
            let thinking = content[start + 10..end].trim().to_string();
            let before = &content[..start];
            let after = &content[end + 11..];
            let clean_content = format!("{}{}", before, after).trim().to_string();
            return (clean_content, Some(thinking));
        }
    }
    
    (content.to_string(), None)
}

// Stubs for non-native builds
#[cfg(not(feature = "native"))]
pub fn check_ollama() -> Result<bool, String> {
    Err("Native feature not enabled".to_string())
}

#[cfg(not(feature = "native"))]
pub fn list_ollama_models() -> Result<Vec<String>, String> {
    Err("Native feature not enabled".to_string())
}

#[cfg(not(feature = "native"))]
pub fn chat(_request: &ChatRequest) -> Result<ChatResponse, String> {
    Err("Native feature not enabled".to_string())
}

/// Streaming chat with Ollama - returns chunks via callback
#[cfg(feature = "native")]
pub fn chat_stream_ollama<F>(
    model: &str,
    messages: &[ChatMessage],
    temperature: f32,
    mut on_chunk: F,
) -> Result<(), String>
where
    F: FnMut(StreamChunk) -> bool, // Return false to stop streaming
{
    use std::io::{BufRead, BufReader};
    
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    
    #[derive(serde::Serialize)]
    struct OllamaStreamRequest {
        model: String,
        messages: Vec<OllamaStreamMessage>,
        stream: bool,
        think: bool,  // Enable thinking for supported models
        options: OllamaStreamOptions,
    }
    
    #[derive(serde::Serialize)]
    struct OllamaStreamMessage {
        role: String,
        content: String,
    }
    
    #[derive(serde::Serialize)]
    struct OllamaStreamOptions {
        temperature: f32,
    }
    
    let ollama_messages: Vec<OllamaStreamMessage> = messages.iter().map(|m| {
        OllamaStreamMessage {
            role: match m.role {
                MessageRole::System => "system".to_string(),
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
            },
            content: m.content.clone(),
        }
    }).collect();
    
    let req_body = OllamaStreamRequest {
        model: model.to_string(),
        messages: ollama_messages,
        stream: true,
        think: true,  // Enable thinking
        options: OllamaStreamOptions { temperature },
    };
    
    let resp = client.post("http://localhost:11434/api/chat")
        .json(&req_body)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if !resp.status().is_success() {
        return Err(format!("Ollama error: {}", resp.status()));
    }
    
    // Read streaming response line by line
    let reader = BufReader::new(resp);
    let mut full_content = String::new();
    let mut full_thinking = String::new();
    
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Read error: {}", e))?;
        if line.is_empty() { continue; }
        
        // Ollama's new thinking API has separate thinking and content fields
        #[derive(serde::Deserialize)]
        struct OllamaStreamChunk {
            message: Option<OllamaChunkMessage>,
            done: bool,
        }
        
        #[derive(serde::Deserialize)]
        struct OllamaChunkMessage {
            #[serde(default)]
            content: String,
            #[serde(default)]
            thinking: Option<String>,
        }
        
        if let Ok(chunk) = serde_json::from_str::<OllamaStreamChunk>(&line) {
            if let Some(msg) = chunk.message {
                // Handle thinking field (new Ollama API)
                if let Some(thinking) = &msg.thinking {
                    if !thinking.is_empty() {
                        full_thinking.push_str(thinking);
                        if !on_chunk(StreamChunk::Thinking(thinking.clone())) {
                            return Ok(());
                        }
                    }
                }
                
                // Handle content field
                if !msg.content.is_empty() {
                    full_content.push_str(&msg.content);
                    if !on_chunk(StreamChunk::Content(msg.content.clone())) {
                        return Ok(());
                    }
                }
            }
            
            if chunk.done {
                // Send done with accumulated content and thinking
                let thinking = if full_thinking.is_empty() { None } else { Some(full_thinking.clone()) };
                on_chunk(StreamChunk::Done { 
                    content: full_content.clone(), 
                    thinking 
                });
                break;
            }
        }
    }
    
    Ok(())
}

/// Stream chunk types
#[derive(Debug, Clone)]
pub enum StreamChunk {
    /// Regular content delta
    Content(String),
    /// Thinking/reasoning delta
    Thinking(String),
    /// Stream completed
    Done { content: String, thinking: Option<String> },
}

#[cfg(not(feature = "native"))]
pub fn chat_stream_ollama<F>(
    _model: &str,
    _messages: &[ChatMessage],
    _temperature: f32,
    _on_chunk: F,
) -> Result<(), String>
where
    F: FnMut(StreamChunk) -> bool,
{
    Err("Native feature not enabled".to_string())
}
