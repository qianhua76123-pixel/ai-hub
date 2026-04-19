# AI Hub

> Local-first desktop app for managing all your AI tools — subscriptions, usage, costs, and task routing in one place.

## Features

- **Zero-config Detection** — Automatically discovers Claude Code, Cursor, Codex CLI, and 48+ AI providers
- **Local Proxy** — Transparent HTTP proxy captures all AI API traffic for usage tracking
- **Cost Tracking** — Per-request cost calculation with cache read/write awareness (28+ models)
- **Smart Routing** — Classifies prompts (code/reasoning/writing/chat) and recommends optimal models
- **Multi-Agent Tasks** — Send the same prompt to multiple providers in parallel, compare results
- **Project Attribution** — Costs automatically attributed to projects via git branch detection
- **ROI Calculator** — Know if your Claude Pro subscription is worth it vs API pricing
- **Conversation Search** — Full-text search across all AI tool histories (FTS5)
- **Health Monitoring** — Real-time provider health with circuit breaker and auto-failover
- **Dark Mode** — System-aware theme with manual override
- **Secure Key Storage** — API keys encrypted via macOS Keychain

## Quick Start

```bash
# Prerequisites: Node.js 18+, Rust 1.70+, pnpm
pnpm install
pnpm tauri dev
```

## Architecture

```
Frontend: React 19 + TypeScript + Tailwind CSS 4 + Recharts
Backend:  Rust + Tauri v2 + SQLite (rusqlite) + Axum (proxy)
Desktop:  macOS (primary), Windows/Linux (planned)
```

### Key Modules

| Module | Purpose |
|--------|---------|
| `proxy.rs` | HTTP proxy with SSE streaming support |
| `scanner.rs` | Auto-detect AI tools from env vars, config files, IDE plugins |
| `traffic.rs` | Parse logs from Claude Code (JSONL), Codex (SQLite), Cursor (DB) |
| `engine.rs` | Task execution with 8 provider APIs |
| `router.rs` | Prompt classification + benchmark-based model recommendation |
| `health.rs` | Circuit breaker, rate limit prediction, background health pings |
| `pricing.rs` | 28+ model prices with cache-aware cost calculation |
| `keystore.rs` | macOS Keychain integration for secure API key storage |
| `presets.rs` | 25+ one-click provider presets |

## Development

```bash
# Run dev server
pnpm tauri dev

# Type check
npx tsc --noEmit

# Rust check + tests
cd src-tauri
cargo check
cargo test --lib

# Build for production
pnpm tauri build
```

## Privacy

- All data stored locally in `~/Library/Application Support/ai-hub/`
- API keys encrypted via macOS Keychain (never stored in plaintext)
- Proxy runs on `127.0.0.1` only — no external access
- Zero telemetry, zero cloud sync, zero third-party data sharing

## License

MIT
