use crate::llm::LLMClient;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io;

// Token counting constants
const MAX_CONTEXT_TOKENS: usize = 200_000; // 200K token limit

#[derive(Debug, Clone)]
pub struct Context {
    content: String,
    metadata: HashMap<String, String>,
}

impl Context {
    pub fn new() -> Self {
        Context {
            content: String::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn add_content(&mut self, content: &str) {
        if !self.content.is_empty() {
            self.content.push_str("\n\n");
        }
        self.content.push_str(content);
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.metadata.clear();
    }

    pub fn get_content(&self) -> &str {
        &self.content
    }

    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    pub fn is_empty(&self) -> bool {
        self.content.trim().is_empty()
    }

    pub fn len(&self) -> usize {
        self.content.len()
    }

}

#[derive(Debug)]
pub struct ContextManager {
    global_context: Context,
    function_contexts: HashMap<String, Context>,
    current_function: Option<String>,
}

impl ContextManager {
    pub fn new() -> Self {
        ContextManager {
            global_context: Context::new(),
            function_contexts: HashMap::new(),
            current_function: None,
        }
    }

    pub fn get_current_context(&self) -> &Context {
        if let Some(func_name) = &self.current_function {
            self.function_contexts
                .get(func_name)
                .unwrap_or(&self.global_context)
        } else {
            &self.global_context
        }
    }

    pub fn get_current_context_mut(&mut self) -> &mut Context {
        if let Some(func_name) = &self.current_function {
            let func_name = func_name.clone();
            self.function_contexts
                .entry(func_name)
                .or_insert_with(Context::new)
        } else {
            &mut self.global_context
        }
    }

    pub fn add_to_context(&mut self, content: &str) {
        self.get_current_context_mut().add_content(content);
    }

    pub fn clear_context(&mut self) {
        self.get_current_context_mut().clear();
    }

    pub fn enter_function(&mut self, function_name: String) {
        self.current_function = Some(function_name.clone());
        self.function_contexts
            .entry(function_name)
            .or_insert_with(Context::new);
    }

    pub fn exit_function(&mut self) {
        self.current_function = None;
    }

    pub fn get_global_context(&self) -> &Context {
        &self.global_context
    }

    pub fn get_function_context(&self, function_name: &str) -> Option<&Context> {
        self.function_contexts.get(function_name)
    }
}

#[derive(Debug, Clone)]
pub enum LLMAction {
    ReadFile { filename: String },
    ClearContext,
    Analyze { content: String },
    Summarize { content: String },
    AddToContext { content: String },
    Comment { content: String }, // Regular markdown paragraph
}

pub struct LLMActionProcessor {
    context_manager: ContextManager,
    llm_client: LLMClient,
    total_tokens_used: usize, // Track actual tokens from API responses
}

impl LLMActionProcessor {
    pub fn new() -> Self {
        LLMActionProcessor {
            context_manager: ContextManager::new(),
            llm_client: LLMClient::new(),
            total_tokens_used: 0,
        }
    }

    pub async fn process_action(&mut self, action: LLMAction) -> io::Result<String> {
        match action {
            LLMAction::ReadFile { filename } => {
                match fs::read_to_string(&filename) {
                    Ok(content) => {
                        self.context_manager
                            .add_to_context(&format!("File: {}\n{}", filename, content));
                        Ok(format!(
                            "[SYS] Read file '{}' into context ({} bytes)",
                            filename,
                            content.len()
                        ))
                    }
                    Err(e) => {
                        let error_msg = format!("Error reading file '{}': {}", filename, e);
                        self.context_manager.add_to_context(&error_msg);
                        Ok(format!("[SYS] {}", error_msg))
                    }
                }
            }

            LLMAction::ClearContext => {
                self.context_manager.clear_context();
                Ok("[SYS] Context cleared".to_string())
            }

            LLMAction::Analyze { content } => {
                self.context_manager
                    .add_to_context(&format!("Analysis request: {}", content));
                let current_context = self.context_manager.get_current_context().get_content();

                if current_context.is_empty() {
                    Ok("[SYS] No context available for analysis".to_string())
                } else {
                    match self
                        .llm_client
                        .analyze_context(current_context, &content)
                        .await
                    {
                        Ok(result) => Ok(result),
                        Err(e) => Ok(format!("[SYS] Analysis error: {}", e)),
                    }
                }
            }

            LLMAction::Summarize { content } => {
                self.context_manager
                    .add_to_context(&format!("Summarization request: {}", content));
                let current_context = self.context_manager.get_current_context().get_content();

                if current_context.is_empty() {
                    Ok("[SYS] No context available for summarization".to_string())
                } else {
                    match self
                        .llm_client
                        .summarize_context(current_context, &content)
                        .await
                    {
                        Ok(result) => Ok(result),
                        Err(e) => Ok(format!("[SYS] Summarization error: {}", e)),
                    }
                }
            }

            LLMAction::AddToContext { content } => {
                self.context_manager.add_to_context(&content);
                Ok(format!(
                    "[SYS] Added to context: {} (Total: {} chars)",
                    content.chars().take(50).collect::<String>(),
                    self.context_manager.get_current_context().len()
                ))
            }

            LLMAction::Comment { content } => {
                // Execute paragraph in agentic style - LLM can perform multiple sequential actions
                self.execute_agentic_paragraph(&content).await
            }
        }
    }

    // Execute a paragraph in agentic style - LLM can perform multiple sequential actions
    async fn execute_agentic_paragraph(&mut self, content: &str) -> io::Result<String> {
        let mut all_results = Vec::new();
        let mut current_request = content.to_string();
        let max_iterations = 5; // Prevent infinite loops

        for iteration in 0..max_iterations {
            let current_context = self
                .context_manager
                .get_current_context()
                .get_content()
                .to_string();

            // Add iteration info for debugging
            if iteration > 0 {
                all_results.push(format!("[SYS] Agentic iteration {}", iteration + 1));
            }

            match self
                .llm_client
                .process_with_tools(&current_context, &current_request)
                .await
            {
                Ok((response, tool_calls, tokens_used)) => {
                    // Add the actual tokens used from API response
                    self.add_token_usage(tokens_used);
                    // Add LLM response
                    if !response.trim().is_empty() {
                        all_results.push(response);
                    }

                    // Execute tool calls and update token count display
                    let mut tool_results = Vec::new();
                    for (tool_name, input) in &tool_calls {
                        match self.execute_tool_call(tool_name, input).await {
                            Ok(tool_result) => {
                                all_results.push(tool_result);
                                // Show updated token count after tool execution
                                let updated_tokens = self.get_token_usage();
                                all_results
                                    .push(format!("[SYS] Context updated: {}", updated_tokens));
                                tool_results
                                    .push(format!("Tool {} executed", tool_name));
                            }
                            Err(e) => {
                                let error_msg = format!("[SYS] Tool execution error: {}", e);
                                all_results.push(error_msg.clone());
                                tool_results.push(format!("Tool {} failed: {}", tool_name, e));
                            }
                        }
                    }

                    // If no tools were called, the LLM is done
                    if tool_calls.is_empty() {
                        break;
                    }

                    // Let the LLM continue only if there were successful tool executions
                    // and we haven't reached iteration limit  
                    let has_successful_tools = tool_results.iter()
                        .any(|r| !r.contains("failed:"));
                    
                    if !has_successful_tools || iteration >= max_iterations - 1 {
                        // Stop if all tools failed or we've reached iteration limit
                        break;
                    }
                    
                    // Prepare next request - ask LLM to continue with the task based on results
                    current_request = format!(
                        "Based on the results: {}. Please continue with the original task or indicate completion.",
                        tool_results.join(", ")
                    );
                }
                Err(e) => {
                    all_results.push(format!("[SYS] LLM processing failed: {}", e));
                    break;
                }
            }
        }

        Ok(all_results.join("\n"))
    }

    // Tool execution for LLM-requested operations
    async fn execute_tool_call(
        &mut self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> Result<String, std::io::Error> {
        match tool_name {
            "read_file" => {
                if let Some(filename) = input.get("filename").and_then(|v| v.as_str()) {
                    let action = crate::context::LLMAction::ReadFile {
                        filename: filename.to_string(),
                    };
                    Box::pin(self.process_action(action)).await
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Missing filename parameter",
                    ))
                }
            }
            "clear_context" => {
                let action = crate::context::LLMAction::ClearContext;
                Box::pin(self.process_action(action)).await
            }
            "add_to_context" => {
                if let Some(content) = input.get("content").and_then(|v| v.as_str()) {
                    let action = crate::context::LLMAction::AddToContext {
                        content: content.to_string(),
                    };
                    Box::pin(self.process_action(action)).await
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "Missing content parameter",
                    ))
                }
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unknown tool: {}", tool_name),
            )),
        }
    }

    pub fn enter_function(&mut self, function_name: String) {
        self.context_manager.enter_function(function_name);
    }

    pub fn exit_function(&mut self) {
        self.context_manager.exit_function();
    }

    pub fn get_context_info(&self) -> String {
        let current = self.context_manager.get_current_context();
        let global = self.context_manager.get_global_context();

        format!(
            "Context: {} chars global, {} chars current",
            global.len(),
            current.len()
        )
    }

    pub fn get_token_usage(&self) -> String {
        format_tokens(self.total_tokens_used, MAX_CONTEXT_TOKENS)
    }
    
    pub fn add_token_usage(&mut self, tokens: usize) {
        self.total_tokens_used += tokens;
    }
}

// Helper function to format token counts with K notation
fn format_tokens(used: usize, total: usize) -> String {
    let format_number = |n: usize| -> String {
        if n >= 1000 {
            format!("{}K", n / 1000)
        } else {
            n.to_string()
        }
    };

    format!("{}/{} TOK", format_number(used), format_number(total))
}

