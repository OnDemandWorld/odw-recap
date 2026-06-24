# Recap Desktop App

Tauri-based desktop application for Recap by ODW.ai.

## Tech Stack

- **Tauri 1.4** (Rust backend + webview frontend)
- **Rust** for backend commands
- **Vanilla JavaScript, HTML, CSS** for the frontend
- **SQLite** for local data storage
- **AES-256-GCM** encryption for data at rest

## Project Structure

```
desktop-app/
├── src/                  # Frontend code
│   ├── index.html
│   ├── main.js
│   └── styles.css
├── src-tauri/            # Rust backend
│   └── src/
│       ├── main.rs
│       ├── audio_input/
│       ├── audio_capture/
│       ├── config/
│       ├── error.rs
│       ├── prompt_manager/
│       ├── storage/
│       ├── summarization/
│       ├── sync/
│       └── transcription/
└── tests/                # Integration and E2E test scaffolding
```

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## Development

```bash
npm install
npm run dev
```

## Build

```bash
npm run build
```

## Test

```bash
cd src-tauri
cargo test
```

## Frontend Notes

The frontend uses the `@tauri-apps/api` package to invoke Rust commands. Available commands include:

- `create_meeting`
- `list_meetings`
- `get_config`
- `import_audio_file`
- `get_supported_audio_formats`

## Status

Core UI and backend implemented. See root `DEVELOPMENT.md` for full project status.
