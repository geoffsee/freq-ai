---
name: coding-standards
description: Coding standards and patterns for development. Use when writing or reviewing code, or when unsure about project conventions.
---

# Project Coding Standards

## General Principles
- **Async Programming**: Prefer asynchronous/non-blocking I/O where appropriate for the project's ecosystem.
- **Error Handling**: Follow project-specific conventions (e.g., `Result` in Rust, `Error` objects in JS/TS).
- **Safety**: Minimize use of unsafe or low-level operations unless strictly necessary.
- **Security**: Audit all operations that interact with sensitive resources (filesystem, network, environment).

## Language-Specific Standards

### Rust
- Use common libraries for the domain (e.g., `tokio` for async, `axum` for web).
- Prefer `std::result::Result` and clear error types.
- Follow `rustfmt` defaults.

### TypeScript/JavaScript
- Prefer standard Web APIs (fetch, Streams, WebCrypto) where available.
- Use consistent indentation and formatting (e.g., 2-space indentation).

## Documentation and Style
- **Comments**: Maintain the existing frequency and style of comments.
- **API Documentation**: Use language-standard formats for documentation (e.g., KDoc for Rust, TSDoc for TS).
