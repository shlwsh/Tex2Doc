# Tex2Doc Browser Extension

A cross-browser WebExtension for converting LaTeX documents to Word (.docx) directly in your browser.

## Features

- **Local Conversion**: Convert using WebAssembly (WASM) engine - your files never leave your device
- **Cloud Conversion**: Upload to Tex2Doc cloud service for complex documents
- **Overleaf Integration**: One-click conversion from Overleaf projects
- **arXiv Support**: Download and convert arXiv papers
- **Account Management**: Sign in to track usage, manage quotas, and access billing
- **Quality Reports**: View detailed conversion quality reports

## Supported Browsers

- Chrome / Chromium-based browsers (Edge, Brave, etc.)
- Firefox
- Safari (macOS and iOS)

## Development

### Prerequisites

- Node.js 18+
- Rust (for WASM compilation)
- wasm-pack

### Setup

```bash
# Install dependencies
npm run extension:install

# Build WASM
npm run build:wasm:extension

# Start development server
npm run extension:dev

# Or for specific browser:
npm run extension:dev:chrome
npm run extension:dev:firefox
```

### Building

```bash
# Build for all browsers
npm run extension:build

# Build for specific browser
npm run extension:build:chrome
npm run extension:build:firefox
npm run extension:build:safari

# Create distribution zip
npm run extension:zip
```

### Testing

```bash
# Run unit tests
npm run extension:test

# Run E2E tests
npm run e2e:browser-extension
```

## Project Structure

```
apps/browser-extension/
├── src/
│   ├── entrypoints/       # Extension entry points
│   │   ├── background.ts  # Background service worker
│   │   ├── popup/         # Popup UI
│   │   ├── options/       # Options/settings page
│   │   ├── sidepanel/     # Side panel dashboard
│   │   └── content/       # Content scripts
│   ├── api/               # API client
│   ├── browser/           # Browser compatibility layer
│   ├── conversion/        # Conversion logic
│   ├── state/             # State management
│   ├── ui/                # UI components
│   └── workers/           # Web workers
├── public/
│   ├── icons/             # Extension icons
│   └── wasm/              # WASM engine
└── tests/                # Tests
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Browser                                │
├─────────────┬─────────────┬─────────────┬─────────────────┤
│   Popup     │   Options   │  SidePanel  │ Content Scripts │
└──────┬──────┴──────┬──────┴──────┬──────┴────────┬────────┘
       │              │             │               │
       └──────────────┴──────┬──────┴───────────────┘
                             │
                    ┌───────┴───────┐
                    │   Background   │
                    │ Service Worker│
                    └───────┬───────┘
                            │
       ┌────────────────────┼────────────────────┐
       │                    │                    │
┌──────▼──────┐     ┌──────▼──────┐    ┌──────▼──────┐
│  IndexedDB  │     │  WASM       │    │  API Client │
│  (Jobs)     │     │  (Local)    │    │  (Cloud)    │
└─────────────┘     └─────────────┘    └──────┬──────┘
                                               │
                                        ┌──────▼──────┐
                                        │  Tex2Doc    │
                                        │  API Server │
                                        └─────────────┘
```

## License

MIT
