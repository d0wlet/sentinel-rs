
# Sentinel (Logen) 🛡️

**A blazing fast, "Zero-Lag" log monitoring tool with a TUI, built in Rust.**

[![Rust](https://img.shields.io/badge/built_with-Rust-d63031?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

> "Tail on steroids."

Sentinel monitors your logs in real-time, parses standard errors **and** JSON logs automatically, visualizes error rates with **Sparklines**, and alerts you via Webhooks (Discord/Slack) when things go wrong—all while consuming minimal resources.

![Sentinel Demo](https://via.placeholder.com/800x400?text=Imagine+Cool+GIF+Here)

## What problem does Sentinel solve?

Traditional tools like `tail -f` show you logs line by line, but they don’t help you understand *patterns*.

Sentinel is built for answering questions like:
- "Are errors increasing right now?"
- "Is this spike real or just noise?"
- "Did my deploy make things worse?"

You run it, and within seconds you see error trends instead of raw text.

When running Sentinel, you get:
- A live TUI with error counters
- Real-time sparklines showing error density
- Automatic detection of JSON and plain-text errors
- Optional webhook alerts when thresholds are exceeded

## 🚀 Key Features

*   **Zero-Lag Architecture**: Decoupled log reading (Tokio) and UI rendering (Ratatui). Your UI never freezes, even at 100,000 lines/second.
*   **Intelligent JSON Parsing**: Automatically detects `{"level": "error"}` or `{"severity": "panic"}` without complex regex configuration. Optimized with typed deserialization.
*   **Safety First**: Rate-limited Webhooks prevent API spam during error spikes.
*   **Robust Tailing**: Uses `linemux` to handle log rotation (logrotate) gracefully.
*   **Visual Intelligence**: See error density over time with TUI Sparklines.

## ⚡ Performance

Sentinel is designed for speed.

| Feature | Implementation | Benefit |
| :--- | :--- | :--- |
| **Parsing** | `RegexSet` + Typed JSON | **O(1)** pattern matching & **Zero-alloc** checks |
| **State** | `AtomicU64` + Lock-Free Sampling | No Mutex contention on hot paths |
| **IO** | `Tokio` Async + `Linemux` | Non-blocking file reading |

## 📦 Installation

```bash
# Clone and build
git clone https://github.com/yourusername/sentinel-rs.git
cd sentinel-rs
cargo install --path .
```

## 🎮 Usage

### 1. Zero-Config Simulation (Try it now!)
Don't have a log file? Sentinel can simulate one for you to demonstrate its power.

```bash
sentinel --simulate
```
*Sit back and watch the sparks fly.*

### What does `--simulate` do?

The simulation mode generates a realistic log stream with periodic error bursts so you can immediately see how Sentinel highlights spikes, parses structured errors, and updates the UI in real time.

### 2. Monitoring a Real File
By default, Sentinel reads `test.log` (configurable in future).
Create a `config.yaml` to define your rules.

`sentinel`

## ⚙️ Configuration

Create a `config.yaml` file in the working directory:

```yaml
# How often to update the UI (ms)
polling_interval_ms: 100

# Your Alerting Hook (Optional)
webhook_url: "https://discord.com/api/webhooks/..."

rules:
  - name: "Database"
    pattern: "(?i)database.*error"
    threshold: 1
```

## 🛡️ License

MIT.
