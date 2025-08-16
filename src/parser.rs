use std::fmt;

#[derive(Debug, Clone)]
pub enum CommandLine {
    Simple(SimpleCommand),
    Pipeline(Vec<SimpleCommand>),
    Background(SimpleCommand),
}

#[derive(Debug, Clone)]
pub struct SimpleCommand {
    pub args: Vec<String>,
    pub redirections: Vec<Redirection>,
}

#[derive(Debug, Clone)]
pub struct Redirection {
    pub redir_type: RedirectionType,
    pub filename: String,
}

#[derive(Debug, Clone)]
pub enum RedirectionType {
    Input,   // <
    Output,  // >
    Append,  // >>
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken(String),
    MissingFilename,
    EmptyCommand,
    InvalidSyntax(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken(token) => write!(f, "Unexpected token: {}", token),
            ParseError::MissingFilename => write!(f, "Missing filename for redirection"),
            ParseError::EmptyCommand => write!(f, "Empty command"),
            ParseError::InvalidSyntax(msg) => write!(f, "Invalid syntax: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    tokens: Vec<String>,
    position: usize,
}

impl Parser {
    pub fn new() -> Self {
        Parser {
            tokens: Vec::new(),
            position: 0,
        }
    }

    pub fn parse(&mut self, input: &str) -> Result<CommandLine, ParseError> {
        self.tokens = self.tokenize(input);
        self.position = 0;

        if self.tokens.is_empty() {
            return Err(ParseError::EmptyCommand);
        }

        // Check for background execution
        let is_background = self.tokens.last() == Some(&"&".to_string());
        if is_background {
            self.tokens.pop();
        }

        // Check for pipeline
        if self.tokens.contains(&"|".to_string()) {
            if is_background {
                return Err(ParseError::InvalidSyntax("Background pipelines not supported".to_string()));
            }
            return self.parse_pipeline();
        }

        // Parse simple command
        let simple_command = self.parse_simple_command()?;
        
        if is_background {
            Ok(CommandLine::Background(simple_command))
        } else {
            Ok(CommandLine::Simple(simple_command))
        }
    }

    fn tokenize(&self, input: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                }
                '"' | '\'' if in_quotes && ch == quote_char => {
                    in_quotes = false;
                    quote_char = ' ';
                }
                ' ' | '\t' if !in_quotes => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                }
                '|' | '&' | '<' | '>' if !in_quotes => {
                    if !current_token.is_empty() {
                        tokens.push(current_token.clone());
                        current_token.clear();
                    }
                    
                    // Handle >> redirection
                    if ch == '>' && chars.peek() == Some(&'>') {
                        chars.next();
                        tokens.push(">>".to_string());
                    } else {
                        tokens.push(ch.to_string());
                    }
                }
                '\\' if !in_quotes => {
                    // Handle escape sequences
                    if let Some(next_ch) = chars.next() {
                        current_token.push(next_ch);
                    }
                }
                _ => {
                    current_token.push(ch);
                }
            }
        }

        if !current_token.is_empty() {
            tokens.push(current_token);
        }

        tokens
    }

    fn parse_pipeline(&mut self) -> Result<CommandLine, ParseError> {
        let mut commands = Vec::new();
        let mut current_args = Vec::new();

        for token in &self.tokens {
            if token == "|" {
                if current_args.is_empty() {
                    return Err(ParseError::InvalidSyntax("Empty command in pipeline".to_string()));
                }
                commands.push(SimpleCommand {
                    args: current_args.clone(),
                    redirections: Vec::new(), // Redirections in pipelines are complex, simplified for now
                });
                current_args.clear();
            } else {
                current_args.push(token.clone());
            }
        }

        if current_args.is_empty() {
            return Err(ParseError::InvalidSyntax("Pipeline ends with |".to_string()));
        }

        commands.push(SimpleCommand {
            args: current_args,
            redirections: Vec::new(),
        });

        Ok(CommandLine::Pipeline(commands))
    }

    fn parse_simple_command(&mut self) -> Result<SimpleCommand, ParseError> {
        let mut args = Vec::new();
        let mut redirections = Vec::new();

        while self.position < self.tokens.len() {
            let token = &self.tokens[self.position];

            match token.as_str() {
                "<" => {
                    self.position += 1;
                    if self.position >= self.tokens.len() {
                        return Err(ParseError::MissingFilename);
                    }
                    redirections.push(Redirection {
                        redir_type: RedirectionType::Input,
                        filename: self.tokens[self.position].clone(),
                    });
                }
                ">" => {
                    self.position += 1;
                    if self.position >= self.tokens.len() {
                        return Err(ParseError::MissingFilename);
                    }
                    redirections.push(Redirection {
                        redir_type: RedirectionType::Output,
                        filename: self.tokens[self.position].clone(),
                    });
                }
                ">>" => {
                    self.position += 1;
                    if self.position >= self.tokens.len() {
                        return Err(ParseError::MissingFilename);
                    }
                    redirections.push(Redirection {
                        redir_type: RedirectionType::Append,
                        filename: self.tokens[self.position].clone(),
                    });
                }
                _ => {
                    args.push(self.expand_variables(token));
                }
            }
            self.position += 1;
        }

        if args.is_empty() {
            return Err(ParseError::EmptyCommand);
        }

        Ok(SimpleCommand { args, redirections })
    }

    fn expand_variables(&self, token: &str) -> String {
        let mut result = String::new();
        let mut chars = token.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                if chars.peek() == Some(&'{') {
                    chars.next(); // consume '{'
                    let mut var_name = String::new();
                    
                    while let Some(ch) = chars.next() {
                        if ch == '}' {
                            break;
                        }
                        var_name.push(ch);
                    }
                    
                    if let Ok(value) = std::env::var(&var_name) {
                        result.push_str(&value);
                    }
                } else {
                    // Simple variable expansion $VAR
                    let mut var_name = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch.is_alphanumeric() || ch == '_' {
                            var_name.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    
                    if !var_name.is_empty() {
                        if let Ok(value) = std::env::var(&var_name) {
                            result.push_str(&value);
                        }
                    } else {
                        result.push('$');
                    }
                }
            } else {
                result.push(ch);
            }
        }

        result
    }
}