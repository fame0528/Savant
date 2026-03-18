---
name: hello-savant
description: Example skill demonstrating Savant skill development. Greet users, format text, count words.
version: "1.0.0"
execution_mode: Wasm
capabilities:
  network_access: false
  filesystem_access: false
  max_memory_mb: 64
  max_execution_time_ms: 5000
---

# Hello Savant Skill

A production-quality example skill demonstrating Savant skill conventions. This skill runs in a WASM sandbox and provides text utilities.

## Usage

When the user says "greet [name]" or "hello [name]", call the `greet` function with the name.

When the user asks to "count words" in a text, call the `word_count` function with the text.

When the user asks to "format text" with a style (uppercase, lowercase, titlecase), call the `format_text` function with the text and style.

## Available Functions

### `greet(name: string)`
Returns a personalized greeting message.
- Input: `{"action": "greet", "name": "World"}`
- Output: `"Hello, World! Welcome to Savant."`

### `word_count(text: string)`
Counts words, characters, and lines in the provided text.
- Input: `{"action": "word_count", "text": "Hello world\nThis is Savant"}`
- Output: JSON with word_count, char_count, line_count

### `format_text(text: string, style: string)`
Formats text in the specified style (uppercase, lowercase, titlecase).
- Input: `{"action": "format_text", "text": "hello world", "style": "uppercase"}`
- Output: `"HELLO WORLD"`

## Integration Notes

This skill follows the OpenClaw SKILL.md specification. It runs in WASM mode by default and can be installed via ClawHub or placed directly in the `skills/` directory.

## Security

This skill:
- Has no network access
- Has no filesystem access
- Runs in isolated WASM sandbox
- Has a 5-second execution timeout
- Uses 64MB maximum memory
