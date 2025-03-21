# ClauChat

A minimal chatbot UI for Anthropic **Claude** models built in Rust.

![clauchat_2i6U7mAKGF](https://github.com/user-attachments/assets/596b12b9-5e2a-4208-871e-c688d15422d3)

## Why ClauChat ? 

I was looking for the following: 
- an <u>open source</u> Ai chat bot UI app
- no priced plan
- no bloat (also don't try to support 100 LLM's)
- no obscure data processing

Since I couldn't find a project that met all these criteria, I decided to create **ClauChat** myself.
I also chose to build it in Rust as a way to deepen my understanding of the language.

## Features

**Hit `Shift+Enter` to send a message**
- [x] UI
- [x] Basic chat interaction with Claude
- [x] Code block formatting
- [x] Real time input **cost** preview and total **cost** display
- [ ] Files as input
- [ ] Set <u>system</u> prompt
- [ ] Set model <u>temperature</u>
- [ ] Conversations history

## Prerequisites

- An Anthropic API key (https://console.anthropic.com/)

## Quick start

```bash
# Clone the repository
git clone https://github.com/yetigit/clauchat.git
cd clauchat

# Build in release mode
cargo run --release
```

## Usage

1. Start the application:

2. Click on the `Settings` button in the top-right corner
3. Enter your Anthropic API key
4. Start chatting and **Hit `Shift+Enter` to send**

## Configuration

The config file is stored in:
- Windows: `%APPDATA%\clauchat\config.json`
- macOS: `~/Library/Application Support/clauchat/config.json`
- Linux: `~/.config/clauchat/config.json`

