use crate::llm::LLMClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::process::Command;

// Token counting constants
const MAX_CONTEXT_TOKENS: usize = 200_000; // 200K token limit

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "user", "assistant", or "system"
    pub content: String,
    pub tokens_used: Option<usize>, // Track tokens used for this message (mainly for assistant messages)
}

impl Message {
    pub fn user(content: String) -> Self {
        Message {
            role: "user".to_string(),
            content,
            tokens_used: None,
        }
    }

    pub fn assistant(content: String) -> Self {
        Message {
            role: "assistant".to_string(),
            content,
            tokens_used: None,
        }
    }

    pub fn assistant_with_tokens(content: String, tokens: usize) -> Self {
        Message {
            role: "assistant".to_string(),
            content,
            tokens_used: Some(tokens),
        }
    }

    pub fn system(content: String) -> Self {
        Message {
            role: "system".to_string(),
            content,
            tokens_used: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConversationHistory {
    messages: Vec<Message>,
    metadata: HashMap<String, String>,
}

impl ConversationHistory {
    pub fn new() -> Self {
        ConversationHistory {
            messages: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn add_user_message(&mut self, content: String) {
        self.add_message(Message::user(content));
    }

    pub fn add_assistant_message(&mut self, content: String) {
        self.add_message(Message::assistant(content));
    }

    pub fn add_system_message(&mut self, content: String) {
        self.add_message(Message::system(content));
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.metadata.clear();
    }

    pub fn get_messages(&self) -> &Vec<Message> {
        &self.messages
    }

    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn total_tokens_used(&self) -> usize {
        self.messages.iter().filter_map(|msg| msg.tokens_used).sum()
    }
}

#[derive(Debug)]
pub struct ContextManager {
    global_history: ConversationHistory,
    function_histories: HashMap<String, ConversationHistory>,
    current_function: Option<String>,
}

impl ContextManager {
    pub fn new() -> Self {
        ContextManager {
            global_history: ConversationHistory::new(),
            function_histories: HashMap::new(),
            current_function: None,
        }
    }

    pub fn get_current_history(&self) -> &ConversationHistory {
        if let Some(func_name) = &self.current_function {
            self.function_histories
                .get(func_name)
                .unwrap_or(&self.global_history)
        } else {
            &self.global_history
        }
    }

    pub fn get_current_history_mut(&mut self) -> &mut ConversationHistory {
        if let Some(func_name) = &self.current_function {
            let func_name = func_name.clone();
            self.function_histories
                .entry(func_name)
                .or_insert_with(ConversationHistory::new)
        } else {
            &mut self.global_history
        }
    }

    pub fn add_user_message(&mut self, content: String) {
        self.get_current_history_mut().add_user_message(content);
    }

    pub fn add_assistant_message(&mut self, content: String) {
        self.get_current_history_mut()
            .add_assistant_message(content);
    }

    pub fn add_assistant_message_with_tokens(&mut self, content: String, tokens: usize) {
        self.get_current_history_mut()
            .add_message(Message::assistant_with_tokens(content, tokens));
    }

    pub fn add_system_message(&mut self, content: String) {
        self.get_current_history_mut().add_system_message(content);
    }

    pub fn clear_context(&mut self) {
        self.get_current_history_mut().clear();
    }

    pub fn enter_function(&mut self, function_name: String) {
        self.current_function = Some(function_name.clone());
        self.function_histories
            .entry(function_name)
            .or_insert_with(ConversationHistory::new);
    }

    pub fn exit_function(&mut self) {
        self.current_function = None;
    }

    pub fn get_global_history(&self) -> &ConversationHistory {
        &self.global_history
    }

    pub fn get_function_history(&self, function_name: &str) -> Option<&ConversationHistory> {
        self.function_histories.get(function_name)
    }
}

#[derive(Debug, Clone)]
pub enum LLMAction {
    Comment { content: String }, // Regular markdown paragraph - the only action we still need
}

pub struct LLMActionProcessor {
    context_manager: ContextManager,
    llm_client: LLMClient,
}

impl LLMActionProcessor {
    pub fn new() -> Self {
        LLMActionProcessor {
            context_manager: ContextManager::new(),
            llm_client: LLMClient::new(),
        }
    }

    pub async fn process_action(&mut self, action: LLMAction) -> io::Result<String> {
        match action {
            LLMAction::Comment { content } => {
                // Execute paragraph in agentic style - LLM can perform multiple sequential actions
                self.execute_agentic_paragraph(&content).await
            }
        }
    }

    // Execute a paragraph in agentic style - LLM can perform multiple sequential actions
    async fn execute_agentic_paragraph(&mut self, content: &str) -> io::Result<String> {
        let mut all_results = Vec::new();
        let max_iterations = 5; // Prevent infinite loops

        // Add the user's request to conversation history
        self.context_manager.add_user_message(content.to_string());

        for iteration in 0..max_iterations {
            let current_history = self
                .context_manager
                .get_current_history()
                .get_messages()
                .clone();

            // Add iteration info for debugging
            if iteration > 0 {
                all_results.push(format!("[SYS] Agentic iteration {}", iteration + 1));
            }

            match self
                .llm_client
                .process_with_tools_and_history(&current_history)
                .await
            {
                Ok((response, tool_calls, tokens_used)) => {
                    // Add LLM response to conversation history with token count
                    if !response.trim().is_empty() {
                        self.context_manager
                            .add_assistant_message_with_tokens(response.clone(), tokens_used);
                        // Format each line with [LLM] prefix for display
                        let display_response = response.lines()
                            .map(|line| format!("[LLM] {}", line))
                            .collect::<Vec<_>>()
                            .join("\n");
                        all_results.push(display_response);
                    }

                    // Execute tool calls and add results to context as user messages
                    let mut tool_results = Vec::new();
                    for (tool_name, input) in &tool_calls {
                        match self.execute_tool_call(tool_name, input).await {
                            Ok(tool_result) => {
                                // Add tool result as user message with tool_result content block
                                let tool_result_message = serde_json::json!({
                                    "type": "tool_result",
                                    "tool_use_id": format!("{}_result", tool_name),
                                    "content": tool_result
                                });

                                self.context_manager
                                    .add_user_message(tool_result_message.to_string());
                                all_results.push(format!("[TOOL] {}: {}", tool_name, tool_result));
                                tool_results.push(format!("Tool {} executed", tool_name));
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
                    let has_successful_tools = tool_results.iter().any(|r| !r.contains("failed:"));

                    if !has_successful_tools || iteration >= max_iterations - 1 {
                        // Stop if all tools failed or we've reached iteration limit
                        break;
                    }
                }
                Err(e) => {
                    all_results.push(format!("[SYS] LLM processing failed: {}", e));
                    break;
                }
            }
        }

        Ok(all_results.join("\n"))
    }

    // Simple direct tool functions
    fn read_file(&self, filename: &str) -> serde_json::Value {
        match fs::read_to_string(filename) {
            Ok(content) => serde_json::json!({
                "success": true,
                "content": content,
                "size": content.len()
            }),
            Err(e) => serde_json::json!({
                "success": false,
                "error": format!("Error reading file '{}': {}", filename, e)
            }),
        }
    }

    fn execute_command(&self, command: &str) -> serde_json::Value {
        match Command::new("sh").arg("-c").arg(command).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let exit_code = output.status.code().unwrap_or(-1);

                serde_json::json!({
                    "success": exit_code == 0,
                    "exit_code": exit_code,
                    "stdout": stdout.to_string(),
                    "stderr": stderr.to_string()
                })
            }
            Err(e) => serde_json::json!({
                "success": false,
                "error": format!("Failed to execute command '{}': {}", command, e)
            }),
        }
    }

    fn clear_context(&mut self) -> serde_json::Value {
        self.context_manager.clear_context();
        serde_json::json!({
            "success": true,
            "message": "Context cleared"
        })
    }

    fn add_to_context(&mut self, content: &str) -> serde_json::Value {
        self.context_manager.add_system_message(content.to_string());
        serde_json::json!({
            "success": true,
            "message": format!("Added to context: {}", content.chars().take(50).collect::<String>())
        })
    }

    // Tool execution for LLM-requested operations
    async fn execute_tool_call(
        &mut self,
        tool_name: &str,
        input: &serde_json::Value,
    ) -> Result<serde_json::Value, std::io::Error> {
        let result = match tool_name {
            "read_file" => {
                #[derive(Deserialize)]
                struct ReadFileInput {
                    filename: String,
                }
                let params: ReadFileInput = serde_json::from_value(input.clone()).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
                })?;
                self.read_file(&params.filename)
            }
            "execute_command" => {
                #[derive(Deserialize)]
                struct ExecuteCommandInput {
                    command: String,
                }
                let params: ExecuteCommandInput =
                    serde_json::from_value(input.clone()).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
                    })?;
                self.execute_command(&params.command)
            }
            "clear_context" => self.clear_context(),
            "add_to_context" => {
                #[derive(Deserialize)]
                struct AddToContextInput {
                    content: String,
                }
                let params: AddToContextInput =
                    serde_json::from_value(input.clone()).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, e.to_string())
                    })?;
                self.add_to_context(&params.content)
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Unknown tool: {}", tool_name),
                ));
            }
        };
        Ok(result)
    }

    pub fn enter_function(&mut self, function_name: String) {
        self.context_manager.enter_function(function_name);
    }

    pub fn exit_function(&mut self) {
        self.context_manager.exit_function();
    }

    pub fn get_context_info(&self) -> String {
        let current = self.context_manager.get_current_history();
        let global = self.context_manager.get_global_history();

        format!(
            "Context: {} messages global, {} messages current",
            global.message_count(),
            current.message_count()
        )
    }

    pub fn get_token_usage(&self) -> String {
        let total_tokens = self
            .context_manager
            .get_current_history()
            .total_tokens_used();
        format_tokens(total_tokens, MAX_CONTEXT_TOKENS)
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
