# AI Features Implementation Plan for Vimgram

## Overview

This document outlines the implementation plan for three AI-powered features in Vimgram:

1. **Vimgram Command Assistant** - Natural language commands for Telegram operations
2. **Integrated Coding Assistant** - Code snippets and debugging within the terminal
3. **Smart Reply Drafting** - AI-powered message suggestions

---

## Architecture

### AI Integration Layer

```
┌─────────────────────────────────────────────────────────────┐
│                      Vimgram UI (Ratatui)                    │
├─────────────────────────────────────────────────────────────┤
│                    Command Mode Extension                    │
│  - /ai command for assistant                                │
│  - /code command for coding help                            │
│  - Reply suggestions overlay                                │
├─────────────────────────────────────────────────────────────┤
│                    AI Service Layer                          │
│  - Command Parser (NLP)                                     │
│  - Code Analysis Engine                                     │
│  - Reply Generator                                          │
├─────────────────────────────────────────────────────────────┤
│                   External AI APIs                           │
│  - OpenAI GPT-4 / Anthropic Claude                          │
│  - (Optional: Local LLM via Ollama for privacy)             │
├─────────────────────────────────────────────────────────────┤
│                   Telegram Client (Grammers)                 │
└─────────────────────────────────────────────────────────────┘
```

---

## Dependencies to Add

```toml
[dependencies]
# AI Client
reqwest = { version = "0.12", features = ["json"] }
serde_json = "1.0"

# NLP / Command Parsing
regex = "1.10"
chrono = "0.4"

# (Optional) Local LLM
# ollama-rs = "0.1"
```

---

## Feature 1: Vimgram Command Assistant

### Use Cases
- Mute notifications: "mute all notifications for 2 hours"
- Message search: "find the last message from Alice about meeting"
- Status change: "set my status to coding"
- Quick actions: "archive all unread messages from today"

### Implementation Plan

#### 1.1 Command Parser (Natural Language → Structured Intent)

**New Module: `src/ai/command_parser.rs`**

```rust
pub enum Intent {
    MuteNotifications { duration: Duration },
    SearchMessages { sender: Option<String>, keywords: Vec<String>, limit: usize },
    SetStatus { status: String },
    ArchiveChats { filters: Vec<ChatFilter> },
    Unrecognized(String),
}

pub struct CommandParser {
    // NLP patterns for intent recognition
}
```

**Approach:**
- Use regex patterns for common command patterns
- Implement fuzzy matching for sender names
- Extract time expressions using chrono-english-like parsing
- Support aliases: "mute" = "silence", "find" = "search", etc.

#### 1.2 Intent Execution Layer

**New Module: `src/ai/command_executor.rs`**

```rust
pub struct CommandExecutor {
    client: TelegramClient,
}

impl CommandExecutor {
    pub async fn execute(&self, intent: Intent) -> Result<ActionResult, Error>;
}
```

**Implementation:**

**Mute Notifications:**
- Store mute state in `src/ai/mute_manager.rs`
- Background task to unmute after duration
- Persist mute state to JSON file
- Skip new message notifications during mute period

**Message Search:**
- Iterate through chat history using `client.iter_messages()`
- Filter by sender (using fuzzy name matching)
- Filter by keyword presence (case-insensitive, supports AND/OR logic)
- Display results in search overlay with context snippets
- Allow jumping to result by pressing Enter

**Set Status:**
- Use Telegram's `client.set_status()` or send to Saved Messages
- Support common statuses: "online", "coding", "away", "busy"

**Archive Chats:**
- Batch operation on filtered dialogs
- Show progress indicator
- Support dry-run mode with preview

#### 1.3 UI Integration

**Command Mode Extension (`src/ui/input.rs`)**
- New `/ai` command to enter AI assistant mode
- Display prompt: "AI Assistant: How can I help? > "
- Send natural language input to Command Parser

**Search Results Overlay (`src/ui/draw.rs`)**
- `draw_search_results()` - Show matched messages with:
  - Sender name
  - Timestamp
  - Message preview (highlighted keywords)
  - Chat name
- Navigation with j/k
- Press Enter to jump to chat and scroll to message

**Mute Status Indicator (`src/ui/draw.rs`)**
- Show mute status in header: "🔇 Muted (remaining: 1h 23m)"
- Press `u` to unmute immediately

#### 1.4 Example Flow

```
User presses: /ai
Prompt: AI Assistant: How can I help? >
User types: find last message from Alice about meeting
AI: [Shows search results overlay]
User presses: Enter
App: Jumps to chat, scrolls to message
```

---

## Feature 2: Integrated Coding Assistant

### Use Cases
- Generate code snippets: "write a Rust function to parse JSON"
- Debug code: "what's wrong with this code: <paste code>"
- Explain code: "explain what this regex does"
- Code review: "review this function for bugs"

### Implementation Plan

#### 2.1 Code Detection & Analysis

**New Module: `src/ai/code_assistant.rs`**

```rust
pub struct CodeAssistant {
    ai_client: AIClient,
}

pub enum CodeTask {
    Generate { language: String, description: String },
    Debug { language: String, code: String },
    Explain { language: String, code: String },
    Review { language: String, code: String },
}
```

**Code Language Detection:**
- Auto-detect from file extensions or common patterns
- Support: Rust, Python, JavaScript, TypeScript, Go, Java, C++, etc.

#### 2.2 Integration with Telegram Messages

**Code Block Detection:**
- Parse markdown code blocks in messages (```lang ... ```)
- Extract code for analysis
- Press `c` on a message with code to open Coding Assistant

**Code Extraction from Chat:**
- Scan last N messages for code blocks
- Allow multi-selection with visual mode (v + j/k)

#### 2.3 UI Components

**New `/code` Command (`src/ui/input.rs`)**
```
/code generate Rust function to parse JSON from API response
/code debug [paste code]
/code explain ```rust ... ```
```

**Code Assistant Overlay (`src/ui/draw.rs`)**
```
┌─────────────────────────────────────────┐
│  Code Assistant                          │
├─────────────────────────────────────────┤
│  Task: Generate Rust function           │
├─────────────────────────────────────────┤
│  Result:                                │
│  ```rust                                │
│  fn parse_json(input: &str) -> ...     │
│  ...                                    │
│  ```                                    │
├─────────────────────────────────────────┤
│  Actions: [y] Copy  [r] Retry  [q] Quit │
└─────────────────────────────────────────┘
```

**Keybindings:**
- `c` on message → Open Code Assistant
- `y` → Copy code to clipboard
- `r` → Regenerate / Retry
- `q` → Close

#### 2.4 AI Integration

**Prompt Engineering:**
- System prompt defines role: "You are a coding assistant"
- Context: Language, description, existing code (if debugging)
- Output format: Markdown code blocks with syntax highlighting

**Streaming Responses:**
- Stream AI response token-by-token
- Update overlay in real-time
- Show typing indicator

---

## Feature 3: Smart Reply Drafting

### Use Cases
- Contextual reply suggestions
- Tone adjustment (formal/casual/professional)
- Auto-reply to common questions
- Summarize and reply to long messages

### Implementation Plan

#### 3.1 Context Gathering

**New Module: `src/ai/reply_generator.rs`**

```rust
pub struct ReplyGenerator {
    ai_client: AIClient,
    chat_history: ChatHistoryBuffer,
}

pub struct ReplyContext {
    chat_id: i64,
    last_n_messages: Vec<Message>,
    sender_name: String,
    tone: ReplyTone,
}
```

**Context Window:**
- Last 10-20 messages for context
- Include message metadata (timestamps, sender)
- Handle group chats (highlight relevant sender)

#### 3.2 Reply Generation

**Tone Options:**
- `Neutral` - Default, balanced
- `Professional` - Formal, polite
- `Casual` - Friendly, relaxed
- `Concise` - Short, direct
- `Detailed` - Thorough explanation

**Reply Types:**
- Direct answer (for questions)
- Acknowledgment (for statements)
- Follow-up question (to continue conversation)
- Summary (for long threads)

#### 3.3 UI Integration

**Suggestion Indicator (`src/ui/draw.rs`)**
- Show when suggestions available: "💡 Press Tab for AI suggestions"
- Show in input line when typing

**Suggestion Overlay (`src/ui/draw.rs`)**
```
┌─────────────────────────────────────────┐
│  AI Reply Suggestions                   │
├─────────────────────────────────────────┤
│  [1] Sure, I can help with that. When   │
│      would you like to meet?            │
│  [2] That sounds great! Let me check    │
│      my schedule and get back to you.   │
│  [3] I'd love to! How about tomorrow    │
│      at 3 PM?                           │
├─────────────────────────────────────────┤
│  Tone: [Neutral ▼]  [r] Regenerate     │
│  Press 1-3 to select, Esc to cancel     │
└─────────────────────────────────────────┘
```

**Keybindings:**
- `Tab` in Insert mode → Show suggestions
- `1-9` → Select suggestion
- `r` → Regenerate with different tone
- `t` → Cycle tone options
- Esc → Cancel

#### 3.4 Trigger Modes

**Automatic:**
- Show suggestions after receiving message
- Debounce to avoid spam
- Learn from user acceptance patterns

**Manual:**
- Press `Tab` anytime in Insert mode
- `/reply` command for explicit request

#### 3.5 Learning & Personalization

**Store preferences:**
`src/ai/preferences.json`
```json
{
  "default_tone": "casual",
  "auto_suggest": true,
  "suggestion_count": 3,
  "frequently_used_responses": [...]
}
```

**Improve over time:**
- Track which suggestions user accepts
- Adjust tone suggestions per chat
- Cache common reply patterns

---

## AI Client Infrastructure

### Unified AI Interface

**New Module: `src/ai/client.rs`**

```rust
pub enum AIProvider {
    OpenAI { api_key: String },
    Anthropic { api_key: String },
    Ollama { base_url: String },
}

pub struct AIClient {
    provider: AIProvider,
    model: String,
}

impl AIClient {
    pub async fn chat(&self, messages: Vec<Message>) -> Result<String>;
    pub async fn stream(&self, messages: Vec<Message>) -> Stream<Result<String>>;
}
```

### Configuration

**Environment Variables:**
```bash
OPENAI_API_KEY=sk-...
ANTHROPIC_API_KEY=sk-ant-...
AI_MODEL=gpt-4  # or claude-3-sonnet
AI_PROVIDER=openai  # or anthropic, ollama
OLLAMA_BASE_URL=http://localhost:11434
```

**Config File:** `~/.config/vimgram/ai_config.json`
```json
{
  "provider": "openai",
  "model": "gpt-4o-mini",
  "default_tone": "casual",
  "max_tokens": 1000,
  "temperature": 0.7
}
```

### Fallback Mechanism
- Primary: OpenAI/Anthropic
- Fallback: Ollama (local, free, slower)
- Offline: Cached responses only

---

## File Structure

```
src/
├── ai/
│   ├── mod.rs
│   ├── client.rs           # AI API client
│   ├── command_parser.rs   # NLP for commands
│   ├── command_executor.rs # Execute intents
│   ├── code_assistant.rs   # Coding help
│   ├── reply_generator.rs  # Smart replies
│   ├── mute_manager.rs     # Notification muting
│   └── preferences.rs       # User preferences
├── ui/
│   ├── input.rs            # Add /ai, /code, Tab handling
│   └── draw.rs             # Add AI overlays
├── app.rs                  # Add AI state
└── main.rs                 # Wire up AI services
```

---

## Implementation Phases

### Phase 1: Core Infrastructure (Week 1)
- [ ] Add AI client module with OpenAI/Anthropic support
- [ ] Create configuration system
- [ ] Implement basic API integration tests

### Phase 2: Command Assistant (Week 2)
- [ ] Implement command parser with regex patterns
- [ ] Build mute manager
- [ ] Implement message search with context
- [ ] Add UI overlays for search results
- [ ] Add mute status indicator

### Phase 3: Coding Assistant (Week 3)
- [ ] Implement code detection from messages
- [ ] Build code assistant module
- [ ] Create code assistant overlay UI
- [ ] Add clipboard integration
- [ ] Implement streaming responses

### Phase 4: Smart Reply Drafting (Week 4)
- [ ] Build context gathering from chat history
- [ ] Implement reply generator with tone options
- [ ] Create suggestion overlay UI
- [ ] Add Tab keybinding
- [ ] Implement preference learning

### Phase 5: Polish & Optimization (Week 5)
- [ ] Add Ollama/local LLM support
- [ ] Implement response caching
- [ ] Add rate limiting
- [ ] Performance optimization
- [ ] Documentation and examples

---

## Testing Strategy

### Unit Tests
- Command parser intent recognition
- Code language detection
- Reply generation logic

### Integration Tests
- End-to-end command execution
- AI API mocking
- State persistence

### Manual Testing Checklist
- [ ] Mute/unmute notifications works
- [ ] Message search finds correct results
- [ ] Code assistant generates valid code
- [ ] Reply suggestions appear and work
- [ ] All AI features handle errors gracefully
- [ ] Fallback to local LLM when API fails

---

## Privacy & Security

### Data Handling
- Only send necessary context to AI
- No API keys in code (use environment variables)
- Store API keys securely (keyring)
- Option to use local LLM (Ollama) for privacy

### Sensitive Data Filtering
- Redact phone numbers, emails, addresses
- Filter out credit card numbers, SSNs
- Allow user to mark chats as "private" (no AI)

---

## Cost Considerations

### API Usage Estimation
- Command parsing: ~100 tokens per command
- Code generation: ~500-1000 tokens per request
- Reply suggestions: ~300-500 tokens per suggestion

### Optimization
- Use smaller models (GPT-4o-mini, Claude 3 Haiku)
- Implement response caching
- Batch operations when possible
- Local LLM option for zero cost

---

## Future Enhancements

### Additional AI Features
- Voice message transcription
- Sentiment analysis for chat insights
- Auto-translate messages
- Chat summarization for long threads
- Meeting scheduling integration
- Image/code analysis

### Advanced NLP
- Intent classification with ML models
- Named entity recognition for better search
- Conversation summarization
- Topic detection and clustering

---

## Conclusion

This plan provides a comprehensive approach to integrating AI features into Vimgram while maintaining:

- **Performance** - Async operations, background tasks
- **Privacy** - Local LLM option, data filtering
- **Usability** - Natural language commands, intuitive UI
- **Extensibility** - Modular architecture for new features

The implementation will be phased, allowing incremental delivery and user feedback at each stage.
