# Recall

A lightweight, **local-first** desktop app for active recall and spaced repetition.

Turn notes, passages, documents, and questions into a recall system. Everything
lives on your machine in a single SQLite file. AI is **optional** — it generates
and assists; the app manages memory. Recall works perfectly with no API key and
no internet connection.

> **Philosophy:** AI generates and assists with recall. The application manages
> memory. The core loop is: _learn → create knowledge → make recall questions →
> recall → evaluate → schedule → recall again._

- **Lightweight & fast** — a tiny native binary (Tauri + Rust), not a web app in a window.
- **Local-first & private** — no accounts, no backend server, no telemetry. Your data never leaves your device unless you explicitly use an AI action.
- **Offline-capable** — create, edit, search, review, schedule, import/export all work with no network.
- **AI-provider agnostic** — OpenAI, Anthropic, Gemini, OpenRouter, Ollama, or any OpenAI-compatible endpoint.

---

## Table of contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Development](#development)
- [Database](#database)
- [AI provider configuration](#ai-provider-configuration)
- [Local Ollama configuration](#local-ollama-configuration)
- [Testing](#testing)
- [Building](#building)
- [Packaging](#packaging)
- [Architecture](#architecture)
- [Security considerations](#security-considerations)
- [Keyboard shortcuts](#keyboard-shortcuts)
- [Import / export](#import--export)

---

## Prerequisites

- **Rust** (stable) via [rustup](https://rustup.rs/) — provides `cargo`.
- **Node.js** 18+ and **npm** (for the frontend tooling only; not a runtime dependency of the shipped app).
- **Platform build tools** for [Tauri 2](https://v2.tauri.app/start/prerequisites/):
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`).
  - **Windows:** Microsoft C++ Build Tools and WebView2 (preinstalled on Windows 11).
  - **Linux:** `webkit2gtk` 4.1, `libayatana-appindicator`, `librsvg`, and standard build essentials. On Debian/Ubuntu:
    ```bash
    sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libssl-dev libayatana-appindicator3-dev librsvg2-dev
    ```

The database uses **bundled SQLite** (compiled into the binary) and HTTP uses
**rustls**, so there is no system SQLite or OpenSSL dependency.

## Installation

```bash
git clone <your-repo-url> recall
cd recall
npm install
```

That installs the frontend dev dependencies. Rust crates are fetched on the
first build.

## Development

Run the app with hot-reloading frontend and a native window:

```bash
npm run tauri dev
```

This starts Vite (port 1420) and launches the Tauri window pointed at it. Rust
changes recompile automatically.

Frontend-only iteration (no native window) is possible with `npm run dev`, but
IPC commands only work inside the Tauri window.

## Database

- Engine: **SQLite** (bundled via `rusqlite`), one file per user.
- Location (created automatically on first launch):
  - **macOS:** `~/Library/Application Support/com.recall.app/recall.db`
  - **Windows:** `%APPDATA%\com.recall.app\recall.db`
  - **Linux:** `~/.local/share/com.recall.app/recall.db`
- **Migrations** live in `src-tauri/migrations/*.sql` and are applied on startup,
  tracked by SQLite's `PRAGMA user_version`. To evolve the schema, add a new
  `NNNN_name.sql` file and register it in `src-tauri/src/db/migrations.rs`.
- Indexes exist on the hot paths (`cards.due_at`, `knowledge_items.updated_at`,
  foreign keys, `reviews.reviewed_at`) so the app stays fast with thousands of cards.

Tables: `knowledge_items`, `cards`, `reviews`, `tags`, `knowledge_item_tags`,
`settings`. Timestamps are stored as integer milliseconds since the Unix epoch;
`cards.interval` is stored in days as a real number (allowing sub-day learning steps).

## AI provider configuration

AI is entirely optional. To enable it: open **Settings → AI provider**, choose a
provider, set the model, paste your API key, toggle **Enable AI**, and click
**Save**. Use **Test connection** to verify.

| Provider | Base URL (default) | Example model |
|---|---|---|
| OpenAI | `https://api.openai.com/v1` | `gpt-4o-mini` |
| Anthropic | `https://api.anthropic.com` | `claude-3-5-haiku-latest` |
| Google Gemini | `https://generativelanguage.googleapis.com` | `gemini-2.0-flash` |
| OpenRouter | `https://openrouter.ai/api/v1` | `openai/gpt-4o-mini` |
| Ollama (local) | `http://localhost:11434/v1` | `llama3.2` |
| Custom (OpenAI-compatible) | _your endpoint_ | _your model_ |

See [`examples/ai-providers.example.json`](examples/ai-providers.example.json)
for a reference of each configuration.

**API keys are never stored in files or the database.** They go straight into
your operating system's secure credential store (macOS Keychain, Windows
Credential Manager, or the Linux Secret Service) and are used only in the Rust
backend to authenticate the provider you configured.

What AI can do (all optional):

- **Chat with your knowledge base** — ask questions in the **Chat** tab and get answers grounded in your own notes (see below).
- **Generate questions** from a passage — you review, edit, select, and only then save.
- **Evaluate a free-recall answer** — advisory feedback; the expected answer is always shown and you make the final call.
- **Explain** an answer.

### Chat with your knowledge base

The **Chat** tab lets you ask questions and get answers based on your notes. It
uses lightweight, fully-local retrieval (no embeddings, no vector database, no
extra tables):

1. Your question is reduced to keywords in Rust.
2. Those keywords retrieve and rank the most relevant knowledge items straight
   from SQLite (title matches weighted higher).
3. The top notes (truncated to a token budget) plus the recent conversation are
   sent to your configured provider with a prompt that instructs it to answer
   **only from your notes** and to say when something isn't covered.
4. The answer is shown with clickable **source** chips linking to the notes used.

Only the retrieval step is fully offline; the answer step uses your provider
(which can be a local Ollama model, keeping everything on-device). If you use a
remote provider, Recall asks for confirmation once per chat before sending.
Conversation history lives in memory only and is never written to disk.

## Local Ollama configuration

Run a completely local, private AI with [Ollama](https://ollama.com):

```bash
ollama serve
ollama pull llama3.2
```

Then in **Settings → AI provider**:

- **Provider:** `Ollama (local)`
- **Base URL:** `http://localhost:11434/v1`
- **API key:** anything (e.g. `ollama`) — it is ignored by Ollama
- **Model:** `llama3.2`

Recall detects local endpoints and tells you plainly that **your content will
not leave this device**.

## Testing

Core logic is covered by tests.

**Rust** (database CRUD/migrations, import/export round-trip, AI request
building, response parsing, and HTTP behavior for success / malformed / auth /
rate-limit / timeout / network failure using a local mock server):

```bash
cd src-tauri
cargo test
```

**TypeScript** (the spaced-repetition scheduler across Again/Hard/Good/Easy,
invariants, and interval formatting):

```bash
npm test
```

## Building

Frontend production bundle only:

```bash
npm run build      # tsc --noEmit && vite build  → dist/
```

Full native app (release):

```bash
npm run tauri build
```

The release profile is tuned for a small binary (`opt-level = "s"`, fat LTO,
stripped).

## Packaging

`npm run tauri build` produces platform installers under
`src-tauri/target/release/bundle/`:

- **macOS:** `.app` bundle and `.dmg` (in `bundle/macos/` and `bundle/dmg/`).
- **Windows:** `.msi` (WiX) and/or `.exe` (NSIS) in `bundle/`.
- **Linux:** `.deb`, `.rpm`, and `.AppImage` in `bundle/`.

Build on each target OS to produce that OS's installers. Code signing/notarization
is optional and configured through Tauri if you have certificates.

## Architecture

Clear boundaries between UI, application logic, persistence, AI, and native code.

```
src/                         # Frontend (TypeScript, no framework)
  main.ts                    # entry: theme, router, pages, keyboard, first-run seed
  api/                       # the ONLY place the UI talks to Rust (typed invoke wrappers)
  ai/                        # AI types (providers themselves live in Rust)
  recall/
    scheduler.ts             # PURE spaced-repetition module (+ scheduler.test.ts)
  db/models.ts               # TS mirrors of the Rust models (snake_case across IPC)
  ui/
    components/              # dom helper, router, modal, toast, tag-input, card-editor, ...
    pages/                   # dashboard, library, knowledge, recall, chat, settings
    styles/global.css        # theming (light/dark/system) + all styles

src-tauri/                   # Native backend (Rust)
  migrations/                # SQL migrations (embedded at compile time)
  src/
    commands/                # Tauri IPC command handlers, grouped by domain
    db/                      # models, migration runner, repository (all SQL)
    ai/                      # provider abstraction: openai-compatible, anthropic, gemini
    security/keychain.rs     # OS keychain wrapper for API keys
    filesystem/              # import/export IO + Markdown/plain-text parsers
    state.rs                 # SQLite connection + lazily-created HTTP client
    error.rs                 # structured, serializable errors ({kind, message, hint})
    lib.rs                   # wires plugins, state, and the command registry
```

Key decisions:

- **Thick backend, thin UI.** All persistence, scheduling persistence, AI HTTP
  calls, keychain access, and file IO happen in Rust. The frontend is pure UI +
  typed IPC clients. This keeps the app secure and native-feeling.
- **The scheduler is a pure, standalone TypeScript module** (`src/recall/scheduler.ts`).
  It takes a card's state + a rating and returns the next state. It is not
  coupled to the UI or DB and can be swapped for a more advanced algorithm (e.g.
  FSRS) without touching callers. The Rust side persists whatever schedule it computes.
- **AI providers live in Rust** because that is where the API key must live.
  Request building and response parsing are pure functions, unit-tested without a network.
- **Import/export uses native dialogs from Rust**, so the webview is granted no
  filesystem or dialog permissions.

## Security considerations

- **API keys** are stored only in the OS keychain, never in SQLite, never in
  files, never logged, and never returned to the frontend (the UI can only ask
  _whether_ a key exists). Keys are sent only to the provider you configured.
- **Minimal Tauri capabilities.** The webview gets `core:default` only. Custom
  IPC commands are the app's entire surface; file dialogs run in Rust.
- **Strict CSP.** `connect-src 'self'` — the webview cannot make external network
  requests; all AI traffic originates in Rust. `object-src 'none'`, `base-uri 'self'`, `form-action 'none'`.
- **SQL injection** is prevented by exclusively using parameterized queries; user
  search input has LIKE wildcards escaped.
- **Untrusted content** (user text and AI output) is rendered via `textContent` /
  text nodes — never `innerHTML` — so it cannot inject markup. Cloze blanks are
  built from DOM nodes, not string interpolation.
- **Input validation** on card types, difficulties, ratings, and schedule values
  happens in Rust before any write.
- **Privacy.** Before content is sent to a remote provider, Recall asks for
  confirmation (configurable). Local providers are detected and labeled as
  staying on-device. No telemetry.
- **Graceful AI failure.** If an AI request fails, the operation is skipped with a
  clear, actionable message — the app never crashes, and all non-AI features keep working.

## Keyboard shortcuts

| Key | Action |
|---|---|
| `N` | New knowledge item |
| `R` | Start a recall session |
| `/` | Focus search (Library) |
| `Space` | Reveal answer (in a session) |
| `Enter` | Submit answer (free recall) |
| `1` `2` `3` `4` | Rate Again / Hard / Good / Easy |
| `Esc` | Close dialog / end session |
| `?` | Show shortcuts |

Shortcuts never fire while you are typing in a field.

## Import / export

- **Export JSON** — a complete, human-readable, **versioned** backup of your
  entire library (items, cards, reviews). See
  [`examples/recall-export-example.json`](examples/recall-export-example.json).
- **Export Markdown** — human-readable notes with `Q:`/`A:` cards.
- **Import** — Recall JSON (full round-trip), Markdown (`# headings` become items,
  `Tags:` lines set tags, `Q:`/`A:` pairs become cards), or plain text (one item).

You own your data. The export format is stable and documented.

---

## License

MIT.
