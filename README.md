# Personaliz Desktop Assistant

Personaliz is a Tauri-based desktop assistant that translates plain English instructions into OpenClaw automation tasks. It features a chat-first interface, background scheduling, and local LLM integration.

## Architecture Overview

- **Frontend**: React-based chat interface with a floating assistant icon. It uses Tauri's `invoke` to communicate with the Rust backend.
- **Backend**: Rust/Tauri core that manages a background scheduler and a Local SQLite database for persistence.
- **Local LLM**: Integrated with Ollama (Phi-3) for first-time interactions and command parsing.
- **Automation Layer**: Wraps OpenClaw CLI and executes custom Playwright scripts for browser automation (e.g., LinkedIn posting).
- **Data Storage**: SQLite (local) stores agent configurations, schedules, logs, and pending approvals.

## Features

### 1. Chat-First Interface
A sleek, modern chat interface to interact with the assistant. Users can create agents simply by typing commands like "Create agent trending" or "Create agent hashtag".

### 2. Agent Management
Visualize and manage active agents, their schedules, and status.

### 3. Background Scheduler
A background cron engine that polls the database every minute to trigger scheduled agent tasks.

### 4. Human-in-the-Loop (Approval Flow)
For sensitive actions like public posting, the assistant creates an approval request. Posting only happens once the user clicks "Approve" in the desktop app.

### 5. Local LLM & Model Switching
Personaliz uses a smart LLM router to ensure privacy and accessibility:
- **On First Install**: The app defaults to a local **Phi-3** model (via Ollama). This allows the assistant to guide you through setup and basic automation without an account.
- **Switching to External APIs**: Once you provide an API key (OpenAI, Claude, etc.) in the **⚙️ Settings** tab, the system automatically switches to the cloud provider for enhanced capabilities.
- **Privacy First**: If you remove your API key, the app gracefully falls back to the local model.

## Model Switching Architecture

The routing logic is handled in the frontend (`App.tsx`):
```javascript
if (user_llm_key) {
  use external API model (e.g., GPT-3.5)
} else {
  use local Phi-3 model (Ollama)
}
```
All model transitions and errors are logged to the local SQLite database for transparency.

### 6. Sandbox Mode
Automation scripts run in a visible browser window (non-headless) for transparency and debugging.

## Setup Instructions

1. **Install Dependencies**:
   ```bash
   pnpm install
   ```
2. **Install Ollama**:
   Ensure [Ollama](https://ollama.com/) is installed and running with the `phi3` model:
   ```bash
   ollama run phi3
   ```
3. **Run the App**:
   ```bash
   pnpm tauri dev
   ```

## Demo Agents

1. **Trending Agent**: Searches for trending topics (mocked) and creates a LinkedIn post for approval.
2. **Hashtag Agent**: Periodically searches for #openclaw on LinkedIn and promotes the repository in comments.

## Log & Observability
View detailed execution logs and approval audit trails directly within the "Activity Logs" and "Approvals" sections of the app.
