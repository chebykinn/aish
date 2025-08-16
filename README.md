# aish - AI-Enhanced Shell

An intelligent shell implementation in Rust that combines traditional shell functionality with AI-powered features for enhanced productivity and automation.

## Features

### Core Shell Functionality
- **REPL (Read-Eval-Print-Loop)**: Interactive command prompt
- **Command execution**: Run external programs and system commands
- **Built-in commands**: Essential shell builtins
- **Command history**: Navigate through previous commands with arrow keys
- **Signal handling**: Ctrl+C and Ctrl+D support

### Execution Modes
- **Interactive mode**: Default REPL interface (run `aish` with no arguments)
- **Command mode**: Execute single command with `-c` flag (`aish -c "command"`)
- **Script mode**: Execute commands from shell script file (`aish script.sh`)
- **Markdown mode**: Execute commands from markdown file (`aish script.md`) - **NEW!**
- **Comment support**: Lines starting with `#` are ignored in script mode

### Built-in Commands
- `exit [code]` - Exit the shell with optional exit code
- `cd [directory]` - Change directory (supports ~, relative, and absolute paths)
- `pwd` - Print current working directory
- `echo [-n] [-e] [text]` - Display text with escape sequence support
- `export VAR=value` - Set environment variables
- `unset VAR` - Remove environment variables
- `env` - Display all environment variables
- `type command` - Show command type (builtin vs external)
- `help` - Display help information
- `history` - Show command history info

### Advanced Features
- **I/O Redirection**: 
  - `command > file` - Redirect stdout to file
  - `command < file` - Redirect stdin from file
  - `command >> file` - Append stdout to file
- **Pipes**: `command1 | command2` - Chain commands together
- **Background processes**: `command &` - Run commands in background
- **Variable expansion**: `$VAR` and `${VAR}` syntax
- **Quote handling**: Support for single and double quotes
- **Escape sequences**: Backslash escaping in commands

### AI-Powered Intelligent Scripting ⚡
- **LLM Integration**: Built-in support for Claude AI (Anthropic) for intelligent script processing
- **Natural Language Instructions**: Write tasks in plain English within markdown scripts AND interactive mode
- **Agentic Execution**: AI can autonomously decide which tools to use and when
- **Context Awareness**: AI maintains context across script execution for intelligent decision-making
- **File Operations**: AI can read, analyze, and process files based on natural language requests
- **Command Execution**: AI can execute shell commands and analyze their output
- **Interactive AI Shell**: Natural language commands work directly in interactive mode
- **Tool Calling**: AI uses function calls to interact with the file system and shell environment

### Markdown Scripting
- **Literate Programming**: Mix documentation, natural language instructions, and executable code
- **Intelligent Paragraphs**: Write plain English instructions that the AI will interpret and execute
- **Code Block Execution**: Execute fenced code blocks marked as shell languages
- **Supported Languages**: `bash`, `shell`, `sh`, `aish`, `zsh`, `fish`, or no language specification
- **Multi-Modal Processing**: Combine AI-processed paragraphs with traditional shell code blocks
- **Headers as Comments**: Lines starting with `#` are treated as non-actionable documentation

## Configuration

### LLM Model Configuration

aish supports multiple Anthropic Claude models. Configure using the `ANTHROPIC_MODEL` environment variable:

**Available models:**
- `claude-3-5-sonnet-20241022` (default - most capable)
- `claude-3-haiku-20240307` (faster, more cost-effective)
- `claude-3-opus-20240229` (most capable for complex tasks)

**API Key Setup:**
```bash
# Required: Set your Anthropic API key
export ANTHROPIC_API_KEY="your_api_key_here"

# Optional: Choose a different model
export ANTHROPIC_MODEL="claude-3-5-sonnet-20241022"
```

**Configuration File (.env):**
```bash
# Create a .env file in the project directory
echo "ANTHROPIC_API_KEY=your_api_key_here" > .env
echo "ANTHROPIC_MODEL=claude-3-5-sonnet-20241022" >> .env
```

## Building and Running

```bash
# Build the shell
cargo build --release

# Run the shell interactively (default mode)
cargo run
# or
./target/release/aish

# Execute a single command with -c flag
./target/release/aish -c "echo 'Hello World'"

# Execute commands from a shell script file
./target/release/aish script.sh

# Execute commands from a markdown file (NEW!)
./target/release/aish script.md

# Show help
./target/release/aish --help
```

## Usage Examples

### Interactive Mode

**Traditional Shell Commands:**
```bash
# Basic commands
aish$ pwd
/home/user/aish

aish$ echo "Hello, World!"
Hello, World!

# Environment variables
aish$ export MY_VAR="test value"
aish$ echo $MY_VAR
test value

# I/O redirection
aish$ echo "Hello" > output.txt
aish$ cat < output.txt
Hello

# Pipes
aish$ ls | grep ".rs"
main.rs

# Background processes
aish$ sleep 10 &
[1] 12345
```

**AI-Enhanced Interactive Mode ⚡**
```bash
# Natural language commands work directly in interactive mode
aish$ list the files in this directory
[AI] 0/200K TOK list the files in this directory
[SYS] Command 'ls' output:
main.rs
README.md
Cargo.toml
src/
[AI] Complete: 1K/200K TOK

aish$ what is the current directory
[AI] 1K/200K TOK what is the current directory  
[SYS] Command 'pwd' output:
/home/user/project
[AI] Complete: 2K/200K TOK

aish$ read the README file and tell me what this project does
[AI] 2K/200K TOK read the README file and tell me what this project does
[SYS] Read file 'README.md' into context (2450 bytes)
[LLM] Based on the README file, this is a Rust project for an AI-enhanced shell called "aish"...
[AI] Complete: 5K/200K TOK

# Mix traditional and AI commands seamlessly
aish$ ls -la
total 48
drwxr-xr-x 5 user user 4096 Dec 15 10:30 .
drwxr-xr-x 3 user user 4096 Dec 15 10:29 ..
-rw-r--r-- 1 user user 2450 Dec 15 10:30 README.md

aish$ analyze the cargo.toml file for dependencies
[AI] 5K/200K TOK analyze the cargo.toml file for dependencies
[SYS] Read file 'Cargo.toml' into context (545 bytes)  
[LLM] The Cargo.toml file shows this project has the following key dependencies:
- rustyline: for command line editing
- clap: for argument parsing
- tokio: for async operations
...
[AI] Complete: 8K/200K TOK
```

### Command Mode (-c flag)
```bash
# Execute single commands
./aish -c "pwd"
./aish -c "echo 'Hello from command mode'"
./aish -c "ls | wc -l"

# Use with complex commands
./aish -c "export VAR=value; echo \$VAR"
```

### Script Mode (file execution)
```bash
# Create a script file
cat > script.sh << 'EOF'
#!/path/to/aish
echo "Starting script..."
pwd
export MY_VAR="script variable"
echo "MY_VAR = $MY_VAR"
echo "Script done!"
EOF

# Execute the script
./aish script.sh
```

### AI-Enhanced Markdown Mode ⚡

Create intelligent scripts that combine natural language instructions with shell commands:

```markdown
# Code Analysis Script

Analyze the main source files in this project.

Read the main.rs file and tell me what it does.

```bash
# Traditional shell commands still work
echo "Project structure:"
find src -name "*.rs" | head -5
```

If there are any issues with the code, read the shell.rs file and suggest improvements.

What are the key components of this shell implementation?
```

**Key Features:**
- **Natural Language**: Write instructions in plain English
- **AI Processing**: Claude AI interprets and executes your requests
- **File Operations**: AI can read, analyze, and process files automatically  
- **Context Retention**: AI remembers previous operations within the script
- **Mixed Mode**: Combine AI instructions with traditional shell code blocks

Execute with: `./aish analysis.aish`

### Traditional Markdown Mode

```markdown
# My Deployment Script

This script deploys our application to the server.

## Pre-deployment checks

```bash
echo "Checking system status..."
uptime
df -h
```

## Main deployment

Now we'll deploy the application:

```shell
echo "Starting deployment..."
export DEPLOY_ENV="production"
echo "Deploying to $DEPLOY_ENV environment"
```

## Post-deployment verification

```bash
echo "Verifying deployment..."
echo "Deployment completed successfully!"
```

Traditional markdown scripts still work perfectly!
```

Execute with: `./aish deployment.md`

## Architecture

The shell is organized into several modules:

- **main.rs**: Entry point and module declarations
- **shell.rs**: Core shell logic, REPL loop, and command execution
- **parser.rs**: Command line parsing and tokenization
- **builtins.rs**: Built-in command implementations
- **markdown.rs**: Markdown parsing and intelligent script processing
- **context.rs**: AI context management and LLM action processing
- **llm.rs**: Anthropic Claude integration and tool calling

### Key Components

1. **Shell struct**: Manages shell state, environment variables, job control, and LLM integration
2. **Parser**: Tokenizes and parses command lines into executable structures
3. **Builtins**: Implements essential shell commands
4. **Command execution**: Handles external process spawning and management
5. **LLM Action Processor**: Manages AI context, processes natural language instructions
6. **Markdown Script Engine**: Parses markdown files and routes content to AI or shell execution
7. **Context Manager**: Maintains file contents, execution state, and AI memory across operations

## Dependencies

- `rustyline`: Provides readline-like functionality for command input
- `nix`: Unix system calls and process management
- `libc`: Low-level system interface
- `clap`: Command line argument parsing
- `pulldown-cmark`: Markdown parsing for intelligent script execution
- `reqwest`: HTTP client for Anthropic API communication
- `serde_json`: JSON serialization/deserialization for API requests
- `tokio`: Async runtime for AI operations and HTTP requests
- `regex`: Pattern matching for text processing

## AI Capabilities

### What the AI Can Do ⚡
- **File Analysis**: Read and analyze source code, configuration files, logs, etc.
- **Content Processing**: Summarize, explain, or extract information from text files
- **Code Understanding**: Explain code functionality, suggest improvements, identify issues
- **Context Awareness**: Remember previous operations within a script session
- **Autonomous Decision Making**: Decide which files to read based on your requests
- **Multi-Step Tasks**: Chain multiple operations together (read → analyze → report)

### AI Tool Functions
- `read_file`: Read files into context for analysis
- `clear_context`: Clear the current AI context
- `add_to_context`: Add information to the AI's working memory

### Current AI Limitations
- **API Dependency**: Requires active internet connection and Anthropic API key
- **Context Size**: Limited by Claude's context window (200K tokens)
- **File Operations Only**: Currently limited to reading files (no writing/editing)
- **No System Commands**: AI cannot execute shell commands (only shell code blocks can)

## Traditional Shell Limitations

Some advanced bash/zsh features are not yet implemented:

- Job control (fg, bg, jobs commands)
- Command substitution (`$(command)` or backticks)
- Globbing/wildcards (*, ?, [])
- Aliases
- Functions
- Conditionals and loops
- Advanced prompt customization
- Tab completion

## License

This project is for educational purposes.