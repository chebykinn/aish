use pulldown_cmark::{Parser, Event, Tag, CodeBlockKind, HeadingLevel};
use std::io;
use crate::context::{LLMAction, LLMActionProcessor};
use regex::Regex;

#[derive(Debug, Clone)]
pub enum MarkdownElement {
    Header(u8, String),     // level, text
    Paragraph(String),      // paragraph text - becomes LLM action
    CodeBlock(Option<String>, String), // language, code - becomes shell execution
    InlineCode(String),     // inline code
    FunctionDeclaration(String, Vec<String>, Vec<MarkdownElement>), // name, params, body
}

pub struct MarkdownScript {
    pub elements: Vec<MarkdownElement>,
}

impl MarkdownScript {
    pub fn parse(content: &str) -> Result<Self, io::Error> {
        // First, handle function declarations
        let (content, functions) = Self::extract_functions(content)?;
        
        let mut elements = Vec::new();
        let parser = Parser::new(&content);
        
        let mut current_paragraph = String::new();
        let mut in_code_block = false;
        let mut code_block_lang = None;
        let mut code_block_content = String::new();
        let mut in_header = false;
        let mut header_level = 0;
        let mut header_text = String::new();
        let mut in_paragraph = false;
        
        for event in parser {
            match event {
                Event::Start(tag) => {
                    match tag {
                        Tag::Heading(level, _, _) => {
                            // Finish current paragraph if any
                            if !current_paragraph.trim().is_empty() {
                                elements.push(MarkdownElement::Paragraph(current_paragraph.trim().to_string()));
                                current_paragraph.clear();
                            }
                            in_paragraph = false;
                            
                            in_header = true;
                            header_level = match level {
                                HeadingLevel::H1 => 1,
                                HeadingLevel::H2 => 2,
                                HeadingLevel::H3 => 3,
                                HeadingLevel::H4 => 4,
                                HeadingLevel::H5 => 5,
                                HeadingLevel::H6 => 6,
                            };
                            header_text.clear();
                        }
                        
                        Tag::Paragraph => {
                            in_paragraph = true;
                            current_paragraph.clear();
                        }
                        
                        Tag::CodeBlock(kind) => {
                            // Finish current paragraph if any
                            if !current_paragraph.trim().is_empty() {
                                elements.push(MarkdownElement::Paragraph(current_paragraph.trim().to_string()));
                                current_paragraph.clear();
                            }
                            in_paragraph = false;
                            
                            in_code_block = true;
                            code_block_lang = match kind {
                                CodeBlockKind::Fenced(lang) => {
                                    if lang.is_empty() {
                                        None
                                    } else {
                                        Some(lang.to_string())
                                    }
                                }
                                CodeBlockKind::Indented => None,
                            };
                            code_block_content.clear();
                        }
                        
                        _ => {} // Other start tags
                    }
                }
                
                Event::End(tag) => {
                    match tag {
                        Tag::Heading(_, _, _) => {
                            if in_header {
                                elements.push(MarkdownElement::Header(header_level, header_text.trim().to_string()));
                                in_header = false;
                                header_text.clear();
                            }
                        }
                        
                        Tag::Paragraph => {
                            if in_paragraph && !current_paragraph.trim().is_empty() {
                                elements.push(MarkdownElement::Paragraph(current_paragraph.trim().to_string()));
                                current_paragraph.clear();
                            }
                            in_paragraph = false;
                        }
                        
                        Tag::CodeBlock(_) => {
                            if in_code_block {
                                elements.push(MarkdownElement::CodeBlock(
                                    code_block_lang.clone(),
                                    code_block_content.clone()
                                ));
                                in_code_block = false;
                                code_block_lang = None;
                                code_block_content.clear();
                            }
                        }
                        
                        _ => {} // Other end tags
                    }
                }
                
                Event::Text(text) => {
                    if in_code_block {
                        code_block_content.push_str(&text);
                    } else if in_header {
                        header_text.push_str(&text);
                    } else if in_paragraph {
                        current_paragraph.push_str(&text);
                    }
                }
                
                Event::SoftBreak | Event::HardBreak => {
                    if in_code_block {
                        code_block_content.push('\n');
                    } else if in_header {
                        header_text.push(' ');
                    } else if in_paragraph {
                        current_paragraph.push(' ');
                    }
                }
                
                Event::Code(code) => {
                    if !in_code_block && !in_header {
                        elements.push(MarkdownElement::InlineCode(code.to_string()));
                    }
                }
                
                _ => {
                    // Handle other markdown elements as needed
                }
            }
        }
        
        // Add any remaining paragraph
        if !current_paragraph.trim().is_empty() {
            elements.push(MarkdownElement::Paragraph(current_paragraph.trim().to_string()));
        }
        
        // Add function declarations
        elements.extend(functions);
        
        Ok(MarkdownScript { elements })
    }

    fn extract_functions(content: &str) -> Result<(String, Vec<MarkdownElement>), io::Error> {
        let func_regex = Regex::new(r"func\s+(\w+)\s*\(([^)]*)\)\s*\{")
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("Regex error: {}", e)))?;
        
        let mut functions = Vec::new();
        let mut remaining_content = content.to_string();
        
        // For now, we'll parse function declarations but not extract their bodies
        // This is a simplified implementation
        for captures in func_regex.captures_iter(content) {
            let func_name = captures[1].to_string();
            let params_str = &captures[2];
            let params: Vec<String> = if params_str.trim().is_empty() {
                Vec::new()
            } else {
                params_str.split(',').map(|s| s.trim().to_string()).collect()
            };
            
            // For now, empty function body - in a complete implementation,
            // we would parse the function body content
            functions.push(MarkdownElement::FunctionDeclaration(func_name, params, Vec::new()));
        }
        
        // Remove function declarations from content for now
        remaining_content = func_regex.replace_all(&remaining_content, "").to_string();
        
        Ok((remaining_content, functions))
    }
    
    pub fn get_llm_actions(&self) -> Vec<LLMAction> {
        let mut actions = Vec::new();
        
        for element in &self.elements {
            match element {
                MarkdownElement::Paragraph(text) => {
                    // Send all paragraphs to LLM as comments - let LLM decide which tools to use
                    actions.push(LLMAction::Comment { content: text.to_string() });
                }
                MarkdownElement::Header(_, _) => {
                    // Headers (lines starting with #) are non-actionable comments/labels
                    // They are skipped and not sent to the LLM for processing
                }
                _ => {} // Code blocks and functions are handled separately
            }
        }
        
        actions
    }
    
    pub fn get_executable_blocks(&self) -> Vec<(Option<String>, String)> {
        let mut executable_blocks = Vec::new();
        
        for element in &self.elements {
            match element {
                MarkdownElement::CodeBlock(lang, code) => {
                    // Consider blocks executable if they have no language specified,
                    // or if they're marked as shell/bash/sh/aish
                    let is_executable = match lang {
                        None => true, // No language specified - assume shell
                        Some(l) => {
                            let lang_lower = l.to_lowercase();
                            matches!(lang_lower.as_str(), 
                                   "shell" | "bash" | "sh" | "aish" | "zsh" | "fish" | ""
                            )
                        }
                    };
                    
                    if is_executable && !code.trim().is_empty() {
                        executable_blocks.push((lang.clone(), code.clone()));
                    }
                }
                _ => {} // Other elements are not executable
            }
        }
        
        executable_blocks
    }

    pub fn get_functions(&self) -> Vec<&MarkdownElement> {
        self.elements.iter()
            .filter(|e| matches!(e, MarkdownElement::FunctionDeclaration(_, _, _)))
            .collect()
    }

    pub fn get_headers(&self) -> Vec<(u8, &String)> {
        self.elements.iter()
            .filter_map(|e| match e {
                MarkdownElement::Header(level, text) => Some((*level, text)),
                _ => None,
            })
            .collect()
    }
    
    // DEPRECATED: Manual parsing removed - LLM now handles all tool decisions via function calling
    // fn parse_paragraph_to_action(text: &str) -> LLMAction {
    //     // All paragraphs now go to LLM as Comment actions
    // }
    
    // DEPRECATED: Filename extraction removed - LLM uses read_file tool directly  
    // fn extract_filename(text: &str) -> Option<String> {
    //     // LLM now handles file reading via function calls
    // }
}

pub fn is_markdown_file(filename: &str) -> bool {
    filename.ends_with(".md") || filename.ends_with(".markdown") || filename.ends_with(".aish")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_llm_actions() {
        let content = r#"
# Test Script

Read the file config.json to understand the configuration.

```bash
echo "Executing shell command"
```

Clear the context and analyze the current situation.

```shell
pwd
ls -la
```

Summarize what we've learned so far.
"#;
        
        let script = MarkdownScript::parse(content).unwrap();
        let actions = script.get_llm_actions();
        let executable = script.get_executable_blocks();
        
        assert_eq!(executable.len(), 2);
        assert!(actions.len() >= 3); // Header, read file, clear/analyze, summarize
    }

    #[test]
    fn test_function_parsing() {
        let content = r#"
func deploy(environment) {
    Check if environment is valid.
    
    ```bash
    echo "Deploying to $environment"
    ```
}
"#;
        
        let script = MarkdownScript::parse(content).unwrap();
        let functions = script.get_functions();
        
        assert_eq!(functions.len(), 1);
    }
}