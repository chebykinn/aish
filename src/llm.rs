use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum LLMError {
    ApiKeyMissing,
    RequestFailed(String),
    ParseError(String),
    NetworkError(reqwest::Error),
}

impl fmt::Display for LLMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LLMError::ApiKeyMissing => write!(f, "ANTHROPIC_API_KEY not found in environment"),
            LLMError::RequestFailed(msg) => write!(f, "API request failed: {}", msg),
            LLMError::ParseError(msg) => write!(f, "Failed to parse response: {}", msg),
            LLMError::NetworkError(e) => write!(f, "Network error: {}", e),
        }
    }
}

impl Error for LLMError {}

impl From<reqwest::Error> for LLMError {
    fn from(err: reqwest::Error) -> Self {
        LLMError::NetworkError(err)
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
    system: Option<String>,
    tools: Option<Vec<Tool>>,
}

#[derive(Serialize)]
struct Tool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: usize,
    output_tokens: usize,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
    name: Option<String>,
    input: Option<Value>,
}

pub struct AnthropicClient {
    client: Client,
    api_key: String,
    model: String,
}

// Model constants for easy switching
pub const CLAUDE_3_5_SONNET: &str = "claude-3-5-sonnet-20241022";
pub const CLAUDE_3_HAIKU: &str = "claude-3-haiku-20240307";
pub const CLAUDE_3_OPUS: &str = "claude-3-opus-20240229";

// Default model
const DEFAULT_MODEL: &str = CLAUDE_3_5_SONNET;

impl AnthropicClient {
    pub fn new() -> Result<Self, LLMError> {
        dotenv::dotenv().ok(); // Load .env file, ignore if it doesn't exist
        
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| LLMError::ApiKeyMissing)?;
        
        let model = env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        Ok(AnthropicClient {
            client: Client::new(),
            api_key,
            model,
        })
    }
    
    pub fn with_model(model: &str) -> Result<Self, LLMError> {
        dotenv::dotenv().ok();
        
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| LLMError::ApiKeyMissing)?;
        
        Ok(AnthropicClient {
            client: Client::new(),
            api_key,
            model: model.to_string(),
        })
    }

    pub async fn analyze_context(&self, context: &str, prompt: &str) -> Result<String, LLMError> {
        let system_prompt = format!(
            "You are an AI assistant helping with shell script analysis and automation. \
             You have access to the following context:\n\n{}\n\n\
             Provide clear, concise responses focused on the specific request. \
             When analyzing files or configurations, highlight key information and potential issues.",
            context
        );

        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 1000,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            system: Some(system_prompt),
            tools: None,
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("Content-Type", "application/json")
            .header("X-API-Key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LLMError::RequestFailed(error_text));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ParseError(e.to_string()))?;

        if let Some(content) = anthropic_response.content.first() {
            if let Some(ref text) = content.text {
                Ok(text.clone())
            } else {
                Err(LLMError::ParseError("No text in content block".to_string()))
            }
        } else {
            Err(LLMError::ParseError("No content in response".to_string()))
        }
    }

    pub async fn summarize_context(&self, context: &str, request: &str) -> Result<String, LLMError> {
        let prompt = format!(
            "Based on the context provided, please summarize: {}\n\n\
             Focus on the most important points and actionable insights.",
            request
        );
        
        self.analyze_context(context, &prompt).await
    }


    pub async fn process_general_request(&self, context: &str, request: &str) -> Result<String, LLMError> {
        let prompt = if context.trim().is_empty() {
            format!("Please help with: {}", request)
        } else {
            format!(
                "Based on the provided context, please help with: {}\n\n\
                 Use the context information to provide a more informed response.",
                request
            )
        };
        
        self.analyze_context(context, &prompt).await
    }

    pub async fn process_with_tools(&self, context: &str, request: &str, processor: &mut crate::context::LLMActionProcessor) -> Result<String, LLMError> {
        let tools = self.get_available_tools();
        
        let system_prompt = format!(
            "You are an AI assistant helping with shell automation and file operations. \
             You have access to these tools:
             - read_file: Read a file into context
             - clear_context: Clear the current context
             - analyze: Analyze content in context  
             - summarize: Summarize content
             - explain: Explain something
             - add_to_context: Add information to context
             
             Current context: {}
             
             Use tools when the user requests file operations, analysis, or context management. \
             Respond naturally and call tools as needed.",
            if context.trim().is_empty() { "No context loaded" } else { context }
        );

        let request_with_tools = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 1000,
            messages: vec![Message {
                role: "user".to_string(),
                content: request.to_string(),
            }],
            system: Some(system_prompt),
            tools: Some(tools),
        };

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("Content-Type", "application/json")
            .header("X-API-Key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request_with_tools)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LLMError::RequestFailed(error_text));
        }

        let anthropic_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ParseError(e.to_string()))?;

        let mut results = Vec::new();
        
        for content_block in &anthropic_response.content {
            match content_block.content_type.as_str() {
                "text" => {
                    if let Some(ref text) = content_block.text {
                        results.push(format!("🤖 {}", text));
                    }
                },
                "tool_use" => {
                    if let (Some(name), Some(input)) = (&content_block.name, &content_block.input) {
                        match self.execute_tool(name, input, processor).await {
                            Ok(tool_result) => results.push(tool_result),
                            Err(e) => results.push(format!("Tool execution error: {}", e)),
                        }
                    }
                },
                _ => {}
            }
        }

        if results.is_empty() {
            Ok("🤖 Processed request".to_string())
        } else {
            Ok(results.join("\n"))
        }
    }

    fn get_available_tools(&self) -> Vec<Tool> {
        use serde_json::json;
        
        vec![
            Tool {
                name: "read_file".to_string(),
                description: "Read a file into the context for analysis".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "filename": {
                            "type": "string", 
                            "description": "Path to the file to read"
                        }
                    },
                    "required": ["filename"]
                }),
            },
            Tool {
                name: "clear_context".to_string(),
                description: "Clear the current context".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "analyze".to_string(),
                description: "Analyze content in the current context".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "What to analyze"
                        }
                    },
                    "required": ["content"]
                }),
            },
            Tool {
                name: "add_to_context".to_string(),
                description: "Add information to the current context".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Content to add to context"
                        }
                    },
                    "required": ["content"]
                }),
            },
        ]
    }

    async fn execute_tool(&self, tool_name: &str, input: &Value, processor: &mut crate::context::LLMActionProcessor) -> Result<String, LLMError> {
        use crate::context::LLMAction;
        
        let action = match tool_name {
            "read_file" => {
                if let Some(filename) = input.get("filename").and_then(|v| v.as_str()) {
                    LLMAction::ReadFile { filename: filename.to_string() }
                } else {
                    return Err(LLMError::ParseError("Missing filename parameter".to_string()));
                }
            },
            "clear_context" => LLMAction::ClearContext,
            "analyze" => {
                if let Some(content) = input.get("content").and_then(|v| v.as_str()) {
                    LLMAction::Analyze { content: content.to_string() }
                } else {
                    return Err(LLMError::ParseError("Missing content parameter".to_string()));
                }
            },
            "add_to_context" => {
                if let Some(content) = input.get("content").and_then(|v| v.as_str()) {
                    LLMAction::AddToContext { content: content.to_string() }
                } else {
                    return Err(LLMError::ParseError("Missing content parameter".to_string()));
                }
            },
            _ => return Err(LLMError::ParseError(format!("Unknown tool: {}", tool_name))),
        };

        match processor.process_action(action).await {
            Ok(result) => Ok(result),
            Err(e) => Err(LLMError::RequestFailed(e.to_string())),
        }
    }
}

// Unified LLM client wrapper that handles both real and mock clients
pub struct LLMClient {
    client_type: ClientType,
    anthropic_client: Option<AnthropicClient>,
}

enum ClientType {
    Anthropic,
    Mock,
}

impl LLMClient {
    pub fn new() -> Self {
        Self::with_model(None)
    }
    
    pub fn with_model(model: Option<&str>) -> Self {
        dotenv::dotenv().ok();
        
        if let Ok(_) = env::var("ANTHROPIC_API_KEY") {
            let client_result = match model {
                Some(m) => AnthropicClient::with_model(m),
                None => AnthropicClient::new(),
            };
            
            match client_result {
                Ok(client) => {
                    let model_name = &client.model;
                    println!("[SYS] Anthropic LLM integration enabled (model: {})", model_name);
                    LLMClient {
                        client_type: ClientType::Anthropic,
                        anthropic_client: Some(client),
                    }
                }
                Err(e) => {
                    println!("[SYS] Anthropic client initialization failed: {}", e);
                    println!("[SYS] Falling back to mock client");
                    LLMClient {
                        client_type: ClientType::Mock,
                        anthropic_client: None,
                    }
                }
            }
        } else {
            println!("[SYS] ANTHROPIC_API_KEY not found, using mock LLM client");
            LLMClient {
                client_type: ClientType::Mock,
                anthropic_client: None,
            }
        }
    }

    pub async fn analyze_context(&self, context: &str, content: &str) -> Result<String, LLMError> {
        match self.client_type {
            ClientType::Anthropic => {
                if let Some(ref client) = self.anthropic_client {
                    match client.analyze_context(context, content).await {
                        Ok(response) => {
                            let prefixed_response = response.lines()
                                .map(|line| format!("[LLM] {}", line))
                                .collect::<Vec<_>>()
                                .join("\n");
                            Ok(prefixed_response)
                        },
                        Err(e) => Ok(format!("[SYS] Analysis failed: {}", e)),
                    }
                } else {
                    Ok("[SYS] No Anthropic client available".to_string())
                }
            }
            ClientType::Mock => {
                let prefixed_response = content.lines()
                    .map(|line| format!("[LLM] [Mock Analysis] {}", line))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(prefixed_response)
            }
        }
    }

    pub async fn summarize_context(&self, context: &str, content: &str) -> Result<String, LLMError> {
        match self.client_type {
            ClientType::Anthropic => {
                if let Some(ref client) = self.anthropic_client {
                    match client.summarize_context(context, content).await {
                        Ok(response) => {
                            let prefixed_response = response.lines()
                                .map(|line| format!("[LLM] {}", line))
                                .collect::<Vec<_>>()
                                .join("\n");
                            Ok(prefixed_response)
                        },
                        Err(e) => Ok(format!("[SYS] Summarization failed: {}", e)),
                    }
                } else {
                    Ok("[SYS] No Anthropic client available".to_string())
                }
            }
            ClientType::Mock => {
                let prefixed_response = content.lines()
                    .map(|line| format!("[LLM] [Mock Summary] {}", line))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(prefixed_response)
            }
        }
    }


    pub async fn process_with_tools(&self, context: &str, content: &str) -> Result<(String, Vec<(String, serde_json::Value)>, usize), LLMError> {
        match self.client_type {
            ClientType::Anthropic => {
                self.process_with_anthropic_tools(context, content).await
            }
            ClientType::Mock => {
                let prefixed_response = content.lines()
                    .map(|line| format!("[LLM] [Mock] {}", line))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok((prefixed_response, Vec::new(), 0)) // Mock returns 0 tokens
            }
        }
    }

    async fn process_with_anthropic_tools(&self, context: &str, content: &str) -> Result<(String, Vec<(String, serde_json::Value)>, usize), LLMError> {
        use serde::Deserialize;
        
        #[derive(Deserialize)]
        struct ToolResponse {
            content: Vec<ToolContentBlock>,
            usage: Option<ToolUsage>,
        }
        
        #[derive(Deserialize)]
        struct ToolUsage {
            input_tokens: usize,
            output_tokens: usize,
        }
        
        #[derive(Deserialize)]
        struct ToolContentBlock {
            #[serde(rename = "type")]
            content_type: String,
            text: Option<String>,
            name: Option<String>,
            input: Option<Value>,
        }

        let tools = vec![
            serde_json::json!({
                "name": "read_file",
                "description": "Read a file into the context for analysis",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "filename": {"type": "string", "description": "Path to the file to read"}
                    },
                    "required": ["filename"]
                }
            }),
            serde_json::json!({
                "name": "clear_context", 
                "description": "Clear the current context",
                "input_schema": {
                    "type": "object",
                    "properties": {}
                }
            }),
            serde_json::json!({
                "name": "add_to_context",
                "description": "Add information to the current context", 
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "content": {"type": "string", "description": "Content to add to context"}
                    },
                    "required": ["content"]
                }
            })
        ];
        
        let context_summary = if context.trim().is_empty() { 
            "No context loaded".to_string() 
        } else { 
            format!("CONTEXT LOADED ({} chars): {}", context.len(), context)
        };
        
        let system_prompt = format!(
            "You are an AI assistant helping with shell automation and file operations. \
             You operate in AGENTIC mode - you can perform multiple sequential actions to complete complex tasks.\n\n\
             Available tools:\n\
             - read_file: Read files into context for analysis\n\
             - clear_context: Clear current context\n\
             - add_to_context: Add information to context\n\n\
             IMPORTANT INSTRUCTIONS:\n\
             1. When given a task, think about what information you need to complete it\n\
             2. Use tools to gather information, then analyze and provide insights\n\
             3. If you need multiple steps, use tools in sequence (each tool call triggers a follow-up)\n\
             4. Only stop calling tools when you have fully completed the task\n\
             5. Be proactive - if a task requires reading files, analysis, or context building, do it automatically\n\
             6. ALWAYS UTILIZE CONTEXT: If context is loaded, use it to answer questions directly\n\n\
             {}",
            context_summary
        );
        
        
        let request = serde_json::json!({
            "model": DEFAULT_MODEL,
            "max_tokens": 1000,
            "messages": [{
                "role": "user",
                "content": content
            }],
            "system": system_prompt,
            "tools": tools
        });
        
        let response = reqwest::Client::new()
            .post("https://api.anthropic.com/v1/messages")
            .header("Content-Type", "application/json")
            .header("X-API-Key", env::var("ANTHROPIC_API_KEY").unwrap_or_default())
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .map_err(|e| LLMError::NetworkError(e))?;
            
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(LLMError::RequestFailed(error_text));
        }
        
        let tool_response: ToolResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ParseError(e.to_string()))?;
        
        let mut results = Vec::new();
        let mut tool_calls = Vec::new();
        
        for content_block in &tool_response.content {
            match content_block.content_type.as_str() {
                "text" => {
                    if let Some(ref text) = content_block.text {
                        // Prefix each line with [LLM]
                        let prefixed_text = text.lines()
                            .map(|line| format!("[LLM] {}", line))
                            .collect::<Vec<_>>()
                            .join("\n");
                        results.push(prefixed_text);
                    }
                },
                "tool_use" => {
                    if let (Some(name), Some(input)) = (&content_block.name, &content_block.input) {
                        tool_calls.push((name.clone(), input.clone()));
                    }
                },
                _ => {}
            }
        }
        
        let response_text = if results.is_empty() {
            "[LLM] Processed request".to_string()
        } else {
            results.join("\n")
        };
        
        // Extract token usage
        let total_tokens = if let Some(usage) = &tool_response.usage {
            usage.input_tokens + usage.output_tokens
        } else {
            0
        };
        
        Ok((response_text, tool_calls, total_tokens))
    }
}

// Utility function to check if Anthropic integration is available
pub fn is_anthropic_available() -> bool {
    dotenv::dotenv().ok();
    env::var("ANTHROPIC_API_KEY").is_ok()
}

// Internal mock client (hidden from main logic)
struct MockLLMClient;