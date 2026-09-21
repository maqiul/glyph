# Glyph

**English** | [中文](./README.zh-CN.md)

A cross-platform desktop productivity toolbox built with **Tauri 2 + Vue 3**. Switch between tools from the left activity bar; all data is processed locally and never uploaded. Current version: **v0.2.2**.

## Features / Tools

Declared in the tool registry at `src/tools.ts` — adding a tool = add an entry here + create its component. Currently includes:

| Tool | Description |
| --- | --- |
| **Markdown** | Read / edit / live-preview Markdown. Powered by markdown-it + highlight.js with outline, task lists, anchors and code highlighting; can inline local / remote / base64 images; the right file-info panel can be shown/hidden; automatic encoding detection. |
| **Screenshot** | Full-screen / region capture with a global hotkey; captures flow into other tools as inline images. |
| **OCR** | Text recognition from images. Dispatched per platform: local inference on Windows/Linux, native Vision on macOS. |
| **Dev Tools** | A collection of developer utilities, including an **HTTP client** (Postman-style: method, URL, headers, body, history) and **RMB amount-to-words** conversion. |
| **PDF** | View / process PDFs (pure-Rust rendering, no external dll). |
| **File Encoding** | Detect and convert text file encodings (GBK / UTF-8, etc.). |
| **Image Toolbox** | Local image format conversion, compression, and more. |

> The UI is bilingual (Chinese / English, in `src/locales/`), follows the system language, and can be switched manually.

## Tech Stack

- **Desktop framework**: Tauri 2 (Rust backend + system WebView)
- **Frontend**: Vue 3.5 (`<script setup>` + TypeScript), Vite 8 (rolldown), Pinia, vue-i18n
- **Editor / rendering**: CodeMirror 6, markdown-it, highlight.js
- **Backend capabilities**: Rust commands under `src-tauri/` (file IO, encoding, screenshot, OCR, PDF, image processing, etc.)

## Prerequisites

- **Node**: `^22.18.0 || >=24.12.0` (see `engines` in `package.json`)
- **pnpm** (this repo manages deps with pnpm; includes `pnpm-lock.yaml` / `pnpm-workspace.yaml`)
- **Rust** (stable) + per-platform Tauri prerequisites (Windows needs WebView2 / MSVC; Linux needs `webkit2gtk`, etc.; see the [official Tauri platform guide](https://tauri.app/start/prerequisites/))

## Development

```sh
pnpm install

# Frontend hot-reload only (browser, without the Tauri host)
pnpm dev

# Full desktop app development (starts Vite + the Tauri host)
pnpm exec tauri dev
```

## Build & Package

```sh
# Production frontend build: type-check (vue-tsc) + vite build
pnpm build

# Build a desktop installer (Windows example; output under src-tauri/target/release/bundle/)
pnpm exec tauri build --bundles nsis
```

> Valid `--bundles` values are platform-specific (Windows: `nsis` / `msi`; macOS: `dmg` / `app`; Linux: `deb` / `rpm` / `appimage`); do not pass `all`.

## Other Scripts

```sh
pnpm type-check   # vue-tsc type checking
pnpm test:unit    # Vitest unit tests
pnpm lint         # oxlint + eslint (--fix)
pnpm format       # Prettier formatting for src/
```

## Release Convention

- Releases are triggered by **pushing a new, incrementing semantic tag** (e.g. `v0.2.1` → `v0.2.2`), which runs the three-platform GitHub Actions CI (`release.yml` on `on.push.tags: 'v*'`); never reuse or force-move an existing tag.
- The tag version must match the in-app version: keep `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` `version` in sync (the `tauri.conf.json` version determines the artifact name, e.g. `Glyph_0.2.2_x64-setup.exe`). The glyph version in `Cargo.lock` is auto-synced by cargo at build time.

## Project Structure

```
.
├── src/                    # Vue frontend
│   ├── components/         # shared components + per-tool components under tools/
│   ├── stores/             # Pinia (settings / persistent)
│   ├── locales/            # Chinese / English strings
│   └── tools.ts            # tool registry
├── src-tauri/              # Tauri (Rust) backend
│   ├── src/                # commands / markdown / ocr / pdf / screenshot modules
│   ├── capabilities/       # permissions and fs scope
│   └── tauri.conf.json     # Tauri config (CSP, bundle targets, etc.)
└── README.md
```

## License

MIT
