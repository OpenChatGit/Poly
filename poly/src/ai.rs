//! AI/LLM Integration for Poly
//! Supports Ollama, OpenAI, Anthropic and custom OpenAI-compatible APIs
//! Includes native tool/function calling support

use serde::{Deserialize, Serialize};

// ============================================================================
// TOOL DEFINITIONS - For function calling / tool use
// ============================================================================

/// Tool definition for function calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name (function name)
    pub name: String,
    /// Description of what the tool does
    pub description: String,
    /// JSON Schema for parameters
    pub parameters: ToolParameters,
}

/// Tool parameters schema (JSON Schema subset)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameters {
    #[serde(rename = "type")]
    pub param_type: String,  // Usually "object"
    pub properties: std::collections::HashMap<String, ToolProperty>,
    #[serde(default)]
    pub required: Vec<String>,
}

/// Individual tool parameter property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProperty {
    #[serde(rename = "type")]
    pub prop_type: String,  // "string", "number", "boolean", "array", "object"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
}

/// Tool call from the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this tool call
    pub id: String,
    /// Tool/function name
    pub name: String,
    /// Arguments as JSON
    pub arguments: serde_json::Value,
}

/// Tool result to send back to the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool call ID this result is for
    pub tool_call_id: String,
    /// Result content (usually JSON string)
    pub content: String,
    /// Whether the tool call was successful
    #[serde(default = "default_true")]
    pub success: bool,
}

fn default_true() -> bool { true }

/// Chat message with tool support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageWithTools {
    pub role: MessageRole,
    pub content: String,
    /// Tool calls made by assistant (only for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool result (only for tool messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResult>,
}

/// Chat response with tool calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponseWithTools {
    /// The generated text content
    pub content: String,
    /// Thinking/reasoning content (if available)
    pub thinking: Option<String>,
    /// Tool calls requested by the model
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Whether the model wants to use tools (requires another round)
    pub wants_tool_use: bool,
    /// Model used
    pub model: String,
    /// Token usage
    pub usage: Option<TokenUsage>,
    /// Provider used
    pub provider: String,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// AI response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiResponse {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

/// AI Provider
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AiProvider {
    #[default]
    Ollama,
    OpenAI,
    Anthropic,
    Custom,
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
    
    /// Tools available for the model to use
    #[serde(default)]
    pub tools: Vec<Tool>,
    
    /// Tool choice: "auto", "none", or specific tool name
    #[serde(default = "default_tool_choice")]
    pub tool_choice: String,
}

fn default_temperature() -> f32 { 0.7 }
fn default_tool_choice() -> String { "auto".to_string() }

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
    
    /// Tool calls requested by the model (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    
    /// Finish reason: "stop", "tool_calls", "length", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
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
#[allow(dead_code)]
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
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        think: bool,
        options: OllamaOptions,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tools: Vec<OllamaTool>,
    }
    
    #[derive(Serialize)]
    struct OllamaMessage {
        role: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_calls: Option<Vec<OllamaToolCall>>,
    }
    
    #[derive(Serialize, Deserialize, Clone)]
    struct OllamaToolCall {
        #[serde(default)]
        id: Option<String>,
        #[serde(rename = "type", default)]
        call_type: Option<String>,
        function: OllamaFunctionCall,
    }
    
    #[derive(Serialize, Deserialize, Clone)]
    struct OllamaFunctionCall {
        name: String,
        #[serde(default)]
        arguments: serde_json::Value,
    }
    
    #[derive(Serialize)]
    struct OllamaTool {
        #[serde(rename = "type")]
        tool_type: String,
        function: OllamaFunction,
    }
    
    #[derive(Serialize)]
    struct OllamaFunction {
        name: String,
        description: String,
        parameters: serde_json::Value,
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
            tool_calls: None,
        }
    }).collect();
    
    // Convert tools to Ollama format
    let tools: Vec<OllamaTool> = request.tools.iter().map(|t| {
        OllamaTool {
            tool_type: "function".to_string(),
            function: OllamaFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: serde_json::to_value(&t.parameters).unwrap_or(serde_json::json!({})),
            },
        }
    }).collect();
    
    let ollama_req = OllamaRequest {
        model: request.model.clone(),
        messages,
        stream: false,
        think: request.enable_thinking,
        options: OllamaOptions {
            temperature: request.temperature,
            num_predict: request.max_tokens,
        },
        tools,
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
        #[serde(default)]
        content: String,
        #[serde(default)]
        thinking: Option<String>,
        #[serde(default)]
        tool_calls: Option<Vec<OllamaResponseToolCall>>,
    }
    
    // Separate struct for response tool calls (different format than request)
    #[derive(Deserialize, Debug)]
    struct OllamaResponseToolCall {
        #[serde(rename = "type", default)]
        call_type: Option<String>,
        function: OllamaResponseFunction,
    }
    
    #[derive(Deserialize, Debug)]
    struct OllamaResponseFunction {
        name: String,
        #[serde(default)]
        arguments: serde_json::Value,
        #[serde(default)]
        index: Option<u32>,
    }
    
    let ollama_resp: OllamaResponse = resp.json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    // First check for native thinking field (new Ollama API)
    // Then fallback to extracting from <think> tags (older format)
    let (content, thinking) = if ollama_resp.message.thinking.is_some() {
        (ollama_resp.message.content.clone(), ollama_resp.message.thinking.clone())
    } else {
        extract_thinking(&ollama_resp.message.content)
    };
    
    // Convert Ollama tool calls to our format
    let tool_calls = ollama_resp.message.tool_calls.map(|calls| {
        calls.into_iter().enumerate().map(|(i, tc)| ToolCall {
            id: format!("call_{}", tc.function.index.unwrap_or(i as u32)),
            name: tc.function.name,
            arguments: tc.function.arguments,
        }).collect()
    });
    
    let has_tool_calls = tool_calls.as_ref().map(|tc: &Vec<ToolCall>| !tc.is_empty()).unwrap_or(false);
    let finish_reason = if has_tool_calls { Some("tool_calls".to_string()) } else { Some("stop".to_string()) };
    
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
        tool_calls,
        finish_reason,
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
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tools: Vec<OpenAITool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_choice: Option<serde_json::Value>,
    }
    
    #[derive(Serialize)]
    struct OpenAIMessage {
        role: String,
        content: String,
    }
    
    #[derive(Serialize)]
    struct OpenAITool {
        #[serde(rename = "type")]
        tool_type: String,
        function: OpenAIFunction,
    }
    
    #[derive(Serialize)]
    struct OpenAIFunction {
        name: String,
        description: String,
        parameters: serde_json::Value,
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
    
    // Convert tools to OpenAI format
    let tools: Vec<OpenAITool> = request.tools.iter().map(|t| {
        OpenAITool {
            tool_type: "function".to_string(),
            function: OpenAIFunction {
                name: t.name.clone(),
                description: t.description.clone(),
                parameters: serde_json::to_value(&t.parameters).unwrap_or(serde_json::json!({})),
            },
        }
    }).collect();
    
    // Tool choice handling
    let tool_choice = if !request.tools.is_empty() {
        match request.tool_choice.as_str() {
            "none" => Some(serde_json::json!("none")),
            "auto" => Some(serde_json::json!("auto")),
            name => Some(serde_json::json!({"type": "function", "function": {"name": name}})),
        }
    } else {
        None
    };
    
    let openai_req = OpenAIRequest {
        model: request.model.clone(),
        messages,
        temperature: request.temperature,
        max_tokens: request.max_tokens,
        stream: false,
        tools,
        tool_choice,
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
        finish_reason: Option<String>,
    }
    
    #[derive(Deserialize)]
    struct OpenAIResponseMessage {
        content: Option<String>,
        tool_calls: Option<Vec<OpenAIToolCall>>,
    }
    
    #[derive(Deserialize)]
    struct OpenAIToolCall {
        id: String,
        function: OpenAIFunctionCall,
    }
    
    #[derive(Deserialize)]
    struct OpenAIFunctionCall {
        name: String,
        arguments: String,
    }
    
    #[derive(Deserialize)]
    struct OpenAIUsage {
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
    }
    
    let openai_resp: OpenAIResponse = resp.json()
        .map_err(|e| format!("Failed to parse response: {}", e))?;
    
    let choice = openai_resp.choices.first();
    let content = choice
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();
    
    let finish_reason = choice.and_then(|c| c.finish_reason.clone());
    
    // Convert OpenAI tool calls to our format
    let tool_calls = choice.and_then(|c| c.message.tool_calls.as_ref()).map(|calls| {
        calls.iter().map(|tc| {
            // Parse arguments from JSON string
            let args = serde_json::from_str(&tc.function.arguments)
                .unwrap_or(serde_json::json!({}));
            ToolCall {
                id: tc.id.clone(),
                name: tc.function.name.clone(),
                arguments: args,
            }
        }).collect()
    });
    
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
        tool_calls,
        finish_reason,
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
        #[serde(skip_serializing_if = "Vec::is_empty")]
        tools: Vec<AnthropicTool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_choice: Option<AnthropicToolChoice>,
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
    
    #[derive(Serialize)]
    struct AnthropicTool {
        name: String,
        description: String,
        input_schema: serde_json::Value,
    }
    
    #[derive(Serialize)]
    struct AnthropicToolChoice {
        #[serde(rename = "type")]
        choice_type: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
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
    
    // Convert tools to Anthropic format
    let tools: Vec<AnthropicTool> = request.tools.iter().map(|t| {
        AnthropicTool {
            name: t.name.clone(),
            description: t.description.clone(),
            input_schema: serde_json::to_value(&t.parameters).unwrap_or(serde_json::json!({})),
        }
    }).collect();
    
    // Tool choice handling
    let tool_choice = if !request.tools.is_empty() {
        match request.tool_choice.as_str() {
            "none" => None,
            "auto" => Some(AnthropicToolChoice { choice_type: "auto".to_string(), name: None }),
            "any" => Some(AnthropicToolChoice { choice_type: "any".to_string(), name: None }),
            name => Some(AnthropicToolChoice { choice_type: "tool".to_string(), name: Some(name.to_string()) }),
        }
    } else {
        None
    };
    
    let anthropic_req = AnthropicRequest {
        model: request.model.clone(),
        messages,
        max_tokens: request.max_tokens.unwrap_or(4096),
        system: system_msg,
        thinking: thinking_config,
        tools,
        tool_choice,
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
        stop_reason: Option<String>,
    }
    
    #[derive(Deserialize)]
    #[serde(tag = "type")]
    enum AnthropicContent {
        #[serde(rename = "text")]
        Text { text: String },
        #[serde(rename = "thinking")]
        Thinking { thinking: String },
        #[serde(rename = "tool_use")]
        ToolUse { id: String, name: String, input: serde_json::Value },
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
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    
    for block in anthropic_resp.content {
        match block {
            AnthropicContent::Text { text } => content.push_str(&text),
            AnthropicContent::Thinking { thinking: t } => thinking = Some(t),
            AnthropicContent::ToolUse { id, name, input } => {
                tool_calls.push(ToolCall {
                    id,
                    name,
                    arguments: input,
                });
            }
        }
    }
    
    let finish_reason = anthropic_resp.stop_reason.clone();
    let tool_calls_opt = if tool_calls.is_empty() { None } else { Some(tool_calls) };
    
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
        tool_calls: tool_calls_opt,
        finish_reason,
    })
}

/// Extract thinking content from <think> tags (for models that use them)
#[allow(dead_code)]
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
                        if !on_chunk(StreamChunk::Thinking { delta: thinking.clone() }) {
                            return Ok(());
                        }
                    }
                }
                
                // Handle content field
                if !msg.content.is_empty() {
                    full_content.push_str(&msg.content);
                    if !on_chunk(StreamChunk::Content { delta: msg.content.clone() }) {
                        return Ok(());
                    }
                }
            }
            
            if chunk.done {
                // Send done with accumulated content and thinking
                let thinking = if full_thinking.is_empty() { None } else { Some(full_thinking.clone()) };
                on_chunk(StreamChunk::Done { 
                    content: full_content.clone(), 
                    thinking,
                    usage: None,
                });
                break;
            }
        }
    }
    
    Ok(())
}

/// Stream chunk types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamChunk {
    /// Regular content delta
    Content { delta: String },
    /// Thinking/reasoning delta
    Thinking { delta: String },
    /// Stream completed
    Done { content: String, thinking: Option<String>, usage: Option<TokenUsage> },
    /// Error occurred
    Error { message: String },
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

// ============================================================================
// STREAM MANAGER - Global state for managing active streams
// ============================================================================

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Stream state
#[derive(Debug, Clone)]
pub struct StreamState {
    pub id: String,
    pub model: String,
    pub chunks: Vec<StreamChunk>,
    pub is_done: bool,
    pub read_index: usize,  // Track how many chunks have been read
}

lazy_static::lazy_static! {
    static ref STREAMS: Arc<Mutex<HashMap<String, StreamState>>> = Arc::new(Mutex::new(HashMap::new()));
}

/// Generate unique stream ID
fn generate_stream_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("stream_{}", timestamp)
}

/// Start a new streaming chat with Ollama
#[cfg(feature = "native")]
pub fn stream_start_ollama(
    model: &str,
    messages: Vec<ChatMessage>,
    temperature: f32,
    think: bool,
) -> Result<String, String> {
    let stream_id = generate_stream_id();
    let stream_id_clone = stream_id.clone();
    
    // Initialize stream state
    {
        let mut streams = STREAMS.lock().map_err(|e| e.to_string())?;
        streams.insert(stream_id.clone(), StreamState {
            id: stream_id.clone(),
            model: model.to_string(),
            chunks: Vec::new(),
            is_done: false,
            read_index: 0,
        });
    }
    
    // Spawn thread to handle streaming
    let model_for_thread = model.to_string();
    std::thread::spawn(move || {
        let result = stream_ollama_internal(&stream_id_clone, &model_for_thread, &messages, temperature, think);
        if let Err(e) = result {
            // Add error chunk
            if let Ok(mut streams) = STREAMS.lock() {
                if let Some(state) = streams.get_mut(&stream_id_clone) {
                    state.chunks.push(StreamChunk::Error { message: e });
                    state.is_done = true;
                }
            }
        }
    });
    
    Ok(stream_id)
}

/// Internal streaming implementation
#[cfg(feature = "native")]
fn stream_ollama_internal(
    stream_id: &str,
    model: &str,
    messages: &[ChatMessage],
    temperature: f32,
    think: bool,
) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    
    #[derive(serde::Serialize)]
    struct OllamaStreamRequest {
        model: String,
        messages: Vec<OllamaStreamMsg>,
        stream: bool,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        think: bool,
        options: OllamaStreamOpts,
    }
    
    #[derive(serde::Serialize)]
    struct OllamaStreamMsg {
        role: String,
        content: String,
    }
    
    #[derive(serde::Serialize)]
    struct OllamaStreamOpts {
        temperature: f32,
    }
    
    let ollama_messages: Vec<OllamaStreamMsg> = messages.iter().map(|m| {
        OllamaStreamMsg {
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
        think,
        options: OllamaStreamOpts { temperature },
    };
    
    let resp = client.post("http://localhost:11434/api/chat")
        .json(&req_body)
        .send()
        .map_err(|e| format!("Request failed: {}", e))?;
    
    if !resp.status().is_success() {
        return Err(format!("Ollama error: {}", resp.status()));
    }
    
    let reader = BufReader::new(resp);
    let mut full_content = String::new();
    let mut full_thinking = String::new();
    let mut prompt_tokens: Option<u32> = None;
    let mut completion_tokens: Option<u32> = None;
    
    for line in reader.lines() {
        let line = line.map_err(|e| format!("Read error: {}", e))?;
        if line.is_empty() { continue; }
        
        #[derive(serde::Deserialize)]
        struct OllamaChunk {
            message: Option<OllamaChunkMsg>,
            done: bool,
            #[serde(default)]
            eval_count: Option<u32>,
            #[serde(default)]
            prompt_eval_count: Option<u32>,
        }
        
        #[derive(serde::Deserialize)]
        struct OllamaChunkMsg {
            #[serde(default)]
            content: String,
            #[serde(default)]
            thinking: Option<String>,
        }
        
        if let Ok(chunk) = serde_json::from_str::<OllamaChunk>(&line) {
            // Store token counts from final chunk
            if chunk.eval_count.is_some() {
                completion_tokens = chunk.eval_count;
            }
            if chunk.prompt_eval_count.is_some() {
                prompt_tokens = chunk.prompt_eval_count;
            }
            
            if let Some(msg) = chunk.message {
                // Handle thinking
                if let Some(thinking) = &msg.thinking {
                    if !thinking.is_empty() {
                        full_thinking.push_str(thinking);
                        add_chunk(stream_id, StreamChunk::Thinking { delta: thinking.clone() });
                    }
                }
                
                // Handle content
                if !msg.content.is_empty() {
                    full_content.push_str(&msg.content);
                    add_chunk(stream_id, StreamChunk::Content { delta: msg.content.clone() });
                }
            }
            
            if chunk.done {
                let thinking = if full_thinking.is_empty() { None } else { Some(full_thinking.clone()) };
                let usage = Some(TokenUsage {
                    prompt_tokens,
                    completion_tokens,
                    total_tokens: None,
                });
                add_chunk(stream_id, StreamChunk::Done { 
                    content: full_content.clone(), 
                    thinking,
                    usage,
                });
                
                // Mark stream as done
                if let Ok(mut streams) = STREAMS.lock() {
                    if let Some(state) = streams.get_mut(stream_id) {
                        state.is_done = true;
                    }
                }
                break;
            }
        }
    }
    
    Ok(())
}

/// Add chunk to stream
fn add_chunk(stream_id: &str, chunk: StreamChunk) {
    if let Ok(mut streams) = STREAMS.lock() {
        if let Some(state) = streams.get_mut(stream_id) {
            state.chunks.push(chunk);
        }
    }
}

/// Poll for new chunks from a stream
pub fn stream_poll(stream_id: &str) -> Result<(Vec<StreamChunk>, bool), String> {
    let mut streams = STREAMS.lock().map_err(|e| e.to_string())?;
    
    let state = streams.get_mut(stream_id)
        .ok_or_else(|| format!("Stream not found: {}", stream_id))?;
    
    // Get unread chunks
    let new_chunks: Vec<StreamChunk> = state.chunks[state.read_index..].to_vec();
    state.read_index = state.chunks.len();
    
    Ok((new_chunks, state.is_done))
}

/// Cancel/stop a stream
pub fn stream_cancel(stream_id: &str) -> Result<(), String> {
    let mut streams = STREAMS.lock().map_err(|e| e.to_string())?;
    streams.remove(stream_id);
    Ok(())
}

/// Clean up completed streams (call periodically)
pub fn stream_cleanup() {
    if let Ok(mut streams) = STREAMS.lock() {
        streams.retain(|_, state| !state.is_done);
    }
}

/// List active streams
pub fn stream_list() -> Vec<String> {
    if let Ok(streams) = STREAMS.lock() {
        streams.keys().cloned().collect()
    } else {
        Vec::new()
    }
}

// Non-native stubs
#[cfg(not(feature = "native"))]
pub fn stream_start_ollama(
    _model: &str,
    _messages: Vec<ChatMessage>,
    _temperature: f32,
    _think: bool,
) -> Result<String, String> {
    Err("Native feature not enabled".to_string())
}
