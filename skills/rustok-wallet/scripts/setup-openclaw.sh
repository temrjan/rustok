#!/usr/bin/env bash
# Setup script for OpenClaw + rustok-wallet skill
# Run this, enter your secrets when prompted, and the agent will start.

set -euo pipefail

echo "==================================="
echo "OpenClaw + rustok-wallet Setup"
echo "==================================="
echo ""

# --- Check dependencies ---
if ! command -v openclaw &> /dev/null; then
    echo "❌ openclaw CLI not found. Installing..."
    npm install -g openclaw-cli
fi

if ! command -v node &> /dev/null; then
    echo "❌ Node.js not found. Please install Node.js 18+ first."
    exit 1
fi

OPENCLAW_BIN=$(command -v openclaw)
echo "✅ OpenClaw found: $OPENCLAW_BIN"
echo ""

# --- Prompt for secrets ---
echo "1. OpenRouter API Key"
echo "   Get it from: https://openrouter.ai/settings/keys"
echo -n "   Enter key: "
read -s OPENROUTER_KEY
echo ""

if [ -z "$OPENROUTER_KEY" ]; then
    echo "❌ OpenRouter key is required."
    exit 1
fi

echo ""
echo "2. Telegram Bot Token"
echo "   Get it from @BotFather: /newbot"
echo -n "   Enter token: "
read -s TELEGRAM_TOKEN
echo ""

if [ -z "$TELEGRAM_TOKEN" ]; then
    echo "❌ Telegram token is required."
    exit 1
fi

echo ""
echo "3. Your Telegram User ID"
echo "   Get it from @userinfobot"
echo -n "   Enter User ID: "
read TELEGRAM_USER_ID

if [ -z "$TELEGRAM_USER_ID" ]; then
    echo "❌ Telegram User ID is required."
    exit 1
fi

echo ""
echo "4. rustok-agent-mcp data directory"
echo "   Where the agent wallet and audit log live."
echo -n "   Enter path [~/.rustok/agent]: "
read RUSTOK_DATA_DIR
RUSTOK_DATA_DIR=${RUSTOK_DATA_DIR:-"$HOME/.rustok/agent"}

echo ""
echo "5. Agent wallet password"
echo "   Used to unlock the wallet. Stored in env var."
echo -n "   Enter password: "
read -s RUSTOK_PASSWORD
echo ""

if [ -z "$RUSTOK_PASSWORD" ]; then
    echo "❌ Wallet password is required."
    exit 1
fi

echo ""
echo "==================================="
echo "Configuring OpenClaw..."
echo "==================================="

# --- Create OpenClaw config ---
mkdir -p "$HOME/.openclaw"

cat > "$HOME/.openclaw/openclaw.json" <<EOF
{
  "gateway": {
    "mode": "local",
    "port": 19001,
    "bind": "loopback",
    "controlUi": {
      "enabled": true
    }
  },
  "models": {
    "providers": {
      "openrouter": {
        "baseUrl": "https://openrouter.ai/api/v1",
        "apiKey": "$OPENROUTER_KEY"
      }
    }
  },
  "commands": {
    "ownerAllowFrom": ["telegram:$TELEGRAM_USER_ID"]
  }
}
EOF

echo "✅ Config written to ~/.openclaw/openclaw.json"

# --- Add Telegram channel ---
echo ""
echo "Adding Telegram channel..."
$OPENCLAW_BIN channels add --channel telegram --token "$TELEGRAM_TOKEN"
echo "✅ Telegram channel added"

# --- Install skill ---
echo ""
echo "Installing rustok-wallet skill..."
SKILL_DIR="$(cd "$(dirname "$0")/.." && pwd)"
$OPENCLAW_BIN skills install "$SKILL_DIR" --force || true
echo "✅ Skill installed from $SKILL_DIR"

# --- Start rustok-agent-mcp ---
echo ""
echo "==================================="
echo "Starting rustok-agent-mcp..."
echo "==================================="

export RUSTOK_AGENT_PASSWORD="$RUSTOK_PASSWORD"

# Check if wallet exists
if [ ! -d "$RUSTOK_DATA_DIR/agent_wallet" ]; then
    echo "⚠️ No wallet found. Creating one..."
    if [ -f "$(dirname "$0")/../../../target/release/rustok-agent-mcp" ]; then
        "$(dirname "$0")/../../../target/release/rustok-agent-mcp" \
            --data-dir "$RUSTOK_DATA_DIR" \
            --create-wallet \
            --port 3000 &
    else
        echo "❌ rustok-agent-mcp binary not found."
        echo "   Build it first: cargo build --release -p rustok-agent-mcp"
        exit 1
    fi
else
    if [ -f "$(dirname "$0")/../../../target/release/rustok-agent-mcp" ]; then
        "$(dirname "$0")/../../../target/release/rustok-agent-mcp" \
            --data-dir "$RUSTOK_DATA_DIR" \
            --port 3000 &
    else
        echo "❌ rustok-agent-mcp binary not found."
        echo "   Build it first: cargo build --release -p rustok-agent-mcp"
        exit 1
    fi
fi

RUSTOK_PID=$!
sleep 3

if kill -0 "$RUSTOK_PID" 2>/dev/null; then
    echo "✅ rustok-agent-mcp running (PID: $RUSTOK_PID)"
else
    echo "❌ rustok-agent-mcp failed to start."
    exit 1
fi

# --- Start OpenClaw gateway ---
echo ""
echo "==================================="
echo "Starting OpenClaw gateway..."
echo "==================================="

$OPENCLAW_BIN gateway start &
GATEWAY_PID=$!
sleep 5

if kill -0 "$GATEWAY_PID" 2>/dev/null; then
    echo "✅ OpenClaw gateway running (PID: $GATEWAY_PID)"
else
    echo "❌ OpenClaw gateway failed to start."
    echo "   Check logs: openclaw logs"
    exit 1
fi

echo ""
echo "==================================="
echo "🎉 Setup complete!"
echo "==================================="
echo ""
echo "OpenClaw Control UI: http://localhost:19001"
echo "MCP Server:           http://localhost:3000"
echo ""
echo "Next steps:"
echo "1. Open Telegram and message your bot"
echo "2. Say: 'What is my wallet balance?'"
echo "3. The agent will call wallet_context → show your balance"
echo ""
echo "To stop:"
echo "  kill $RUSTOK_PID     # MCP server"
echo "  kill $GATEWAY_PID    # OpenClaw gateway"
