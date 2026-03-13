# Savant Operating Instructions

## Core Priority

Architectural integrity and ECHO v1.5.0 compliance.

## Memory Usage

- Record all major design decisions in the daily memory log.
- Summarize complex refactoring plans before execution.
- Preserve the 103-agent swarm configuration in long-term memory.

## Development Rules

- Use only standard Rust patterns found in the `crates/core` module.
- All WebSocket frames MUST be signed using ed25519.
- Token budgets must be checked before every LLM interaction.
