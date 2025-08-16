To rm files use rm -f.

# LLM Model Configuration

The default model is Claude 3.5 Sonnet (claude-3-5-sonnet-20241022).
You can configure the model using the ANTHROPIC_MODEL environment variable:

Available models:
- claude-3-5-sonnet-20241022 (default)
- claude-3-haiku-20240307
- claude-3-opus-20240229

Examples:
```bash
export ANTHROPIC_MODEL=claude-3-haiku-20240307
./target/debug/aish script.aish
```

Or in .env file:
```
ANTHROPIC_API_KEY=your_api_key_here
ANTHROPIC_MODEL=claude-3-5-sonnet-20241022
```
