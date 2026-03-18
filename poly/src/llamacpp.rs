//! llama.cpp Integration for Poly
//! Provides native GGUF model support without external dependencies

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

// ============================================================================
// TYPES
// ============================================================================

/// llama.cpp model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaCppConfig {
    /// Path to GGUF model file
    pub model_path: PathBuf,
    /// Number of GPU layers to offload (-1 for all)
    pub n_gpu_layers: i32,
    /// Context size
    pub n_ctx: u32,
    /// Batch size
    pub n_batch: u32,
    /// Number of threads
    pub n_threads: Option<u32>,
}

impl Default for LlamaCppConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::new(),
            n_gpu_layers: -1, // Use all GPU layers by default
            n_ctx: 4096,
            n_batch: 512,
            n_threads: None, // Auto-detect
        }
    }
}

/// Chat request for llama.cpp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaCppChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Chat response from llama.cpp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaCppChatResponse {
    pub content: String,
    pub model: String,
    pub tokens_generated: u32,
    pub tokens_per_second: f32,
}

/// Model info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub quantization: String,
    pub loaded: bool,
}

// ============================================================================
// MODEL MANAGER
// ============================================================================

/// Manages llama.cpp models
pub struct ModelManager {
    models_dir: PathBuf,
    loaded_models: HashMap<String, ModelInfo>,
}

impl ModelManager {
    /// Create new model manager
    pub fn new(models_dir: PathBuf) -> Self {
        // Create models directory if it doesn't exist
        if !models_dir.exists() {
            let _ = std::fs::create_dir_all(&models_dir);
        }
        
        Self {
            models_dir,
            loaded_models: HashMap::new(),
        }
    }
    
    /// List available models
    pub fn list_models(&self) -> Result<Vec<ModelInfo>, String> {
        let mut models = Vec::new();
        
        let entries = std::fs::read_dir(&self.models_dir)
            .map_err(|e| format!("Failed to read models directory: {}", e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("gguf") {
                let name = path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                
                let size_bytes = entry.metadata()
                    .map(|m| m.len())
                    .unwrap_or(0);
                
                // Try to detect quantization from filename
                let quantization = detect_quantization(&name);
                
                models.push(ModelInfo {
                    name: name.clone(),
                    path: path.clone(),
                    size_bytes,
                    quantization,
                    loaded: self.loaded_models.contains_key(&name),
                });
            }
        }
        
        Ok(models)
    }
    
    /// Get model path
    pub fn get_model_path(&self, model_name: &str) -> Option<PathBuf> {
        let path = self.models_dir.join(format!("{}.gguf", model_name));
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }
    
    /// Download model from HuggingFace
    #[cfg(feature = "native")]
    pub fn download_from_hf(
        &self,
        repo: &str,
        filename: &str,
        progress_callback: Option<Box<dyn Fn(u64, u64)>>,
    ) -> Result<PathBuf, String> {
        use std::io::Write;
        
        let url = format!("https://huggingface.co/{}/resolve/main/{}", repo, filename);
        let dest_path = self.models_dir.join(filename);
        
        println!("Downloading {} from {}", filename, repo);
        println!("URL: {}", url);
        
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(3600)) // 1 hour timeout
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
        
        let mut response = client.get(&url)
            .send()
            .map_err(|e| format!("Failed to download: {}", e))?;
        
        if !response.status().is_success() {
            return Err(format!("Download failed with status: {}", response.status()));
        }
        
        let total_size = response.content_length().unwrap_or(0);
        println!("Total size: {} MB", total_size / 1024 / 1024);
        
        let mut file = std::fs::File::create(&dest_path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        
        let mut downloaded: u64 = 0;
        let mut buffer = [0; 8192];
        
        loop {
            let bytes_read = std::io::Read::read(&mut response, &mut buffer)
                .map_err(|e| format!("Failed to read: {}", e))?;
            
            if bytes_read == 0 {
                break;
            }
            
            file.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Failed to write: {}", e))?;
            
            downloaded += bytes_read as u64;
            
            if let Some(ref callback) = progress_callback {
                callback(downloaded, total_size);
            }
            
            // Print progress every 10MB
            if downloaded % (10 * 1024 * 1024) == 0 {
                let progress = if total_size > 0 {
                    (downloaded as f64 / total_size as f64 * 100.0) as u32
                } else {
                    0
                };
                println!("Downloaded: {} MB ({}%)", downloaded / 1024 / 1024, progress);
            }
        }
        
        println!("Download complete: {}", dest_path.display());
        Ok(dest_path)
    }
}

/// Detect quantization from model filename
fn detect_quantization(filename: &str) -> String {
    let lower = filename.to_lowercase();
    
    if lower.contains("q2_k") { "Q2_K".to_string() }
    else if lower.contains("q3_k_s") { "Q3_K_S".to_string() }
    else if lower.contains("q3_k_m") { "Q3_K_M".to_string() }
    else if lower.contains("q3_k_l") { "Q3_K_L".to_string() }
    else if lower.contains("q4_0") { "Q4_0".to_string() }
    else if lower.contains("q4_1") { "Q4_1".to_string() }
    else if lower.contains("q4_k_s") { "Q4_K_S".to_string() }
    else if lower.contains("q4_k_m") { "Q4_K_M".to_string() }
    else if lower.contains("q5_0") { "Q5_0".to_string() }
    else if lower.contains("q5_1") { "Q5_1".to_string() }
    else if lower.contains("q5_k_s") { "Q5_K_S".to_string() }
    else if lower.contains("q5_k_m") { "Q5_K_M".to_string() }
    else if lower.contains("q6_k") { "Q6_K".to_string() }
    else if lower.contains("q8_0") { "Q8_0".to_string() }
    else if lower.contains("f16") { "F16".to_string() }
    else if lower.contains("f32") { "F32".to_string() }
    else { "Unknown".to_string() }
}

// ============================================================================
// LLAMA.CPP SERVER CLIENT (for quick integration)
// ============================================================================

/// Client for llama.cpp server (OpenAI-compatible API)
pub struct LlamaCppServerClient {
    base_url: String,
}

impl LlamaCppServerClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
    
    /// Check if server is running
    #[cfg(feature = "native")]
    pub fn check_server(&self) -> Result<bool, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build()
            .map_err(|e| e.to_string())?;
        
        match client.get(format!("{}/health", self.base_url)).send() {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
    
    /// Chat with llama.cpp server
    #[cfg(feature = "native")]
    pub fn chat(&self, request: &LlamaCppChatRequest) -> Result<LlamaCppChatResponse, String> {
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
                role: m.role.clone(),
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
        
        let resp = client.post(format!("{}/v1/chat/completions", self.base_url))
            .json(&openai_req)
            .send()
            .map_err(|e| format!("Request failed: {}", e))?;
        
        if !resp.status().is_success() {
            return Err(format!("Server error: {}", resp.status()));
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
            content: String,
        }
        
        #[derive(Deserialize)]
        struct OpenAIUsage {
            completion_tokens: u32,
        }
        
        let openai_resp: OpenAIResponse = resp.json()
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        
        let content = openai_resp.choices.first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();
        
        let tokens_generated = openai_resp.usage
            .map(|u| u.completion_tokens)
            .unwrap_or(0);
        
        Ok(LlamaCppChatResponse {
            content,
            model: request.model.clone(),
            tokens_generated,
            tokens_per_second: 0.0, // Not available from OpenAI API
        })
    }
}

// Stubs for non-native builds
#[cfg(not(feature = "native"))]
impl LlamaCppServerClient {
    pub fn check_server(&self) -> Result<bool, String> {
        Err("Native feature not enabled".to_string())
    }
    
    pub fn chat(&self, _request: &LlamaCppChatRequest) -> Result<LlamaCppChatResponse, String> {
        Err("Native feature not enabled".to_string())
    }
}

#[cfg(not(feature = "native"))]
impl ModelManager {
    pub fn download_from_hf(
        &self,
        _repo: &str,
        _filename: &str,
        _progress_callback: Option<Box<dyn Fn(u64, u64)>>,
    ) -> Result<PathBuf, String> {
        Err("Native feature not enabled".to_string())
    }
}

// ============================================================================
// POPULAR MODELS
// ============================================================================

/// Popular GGUF models for quick download
pub const POPULAR_MODELS: &[(&str, &str, &str, &str)] = &[
    // (name, repo, filename, description)
    ("Llama 3.2 3B Q4", "bartowski/Llama-3.2-3B-Instruct-GGUF", "Llama-3.2-3B-Instruct-Q4_K_M.gguf", "Fast, efficient model for general tasks (2GB)"),
    ("Qwen 2.5 7B Q4", "Qwen/Qwen2.5-7B-Instruct-GGUF", "qwen2.5-7b-instruct-q4_k_m.gguf", "Excellent reasoning and coding (4.4GB)"),
    ("Phi-3 Mini Q4", "microsoft/Phi-3-mini-4k-instruct-gguf", "Phi-3-mini-4k-instruct-q4.gguf", "Small but powerful (2.3GB)"),
    ("Llama 3.2 1B Q4", "bartowski/Llama-3.2-1B-Instruct-GGUF", "Llama-3.2-1B-Instruct-Q4_K_M.gguf", "Ultra-fast, tiny model (0.7GB)"),
    ("Gemma 2 2B Q4", "bartowski/gemma-2-2b-it-GGUF", "gemma-2-2b-it-Q4_K_M.gguf", "Google's efficient model (1.6GB)"),
];
