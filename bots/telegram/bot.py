#!/usr/bin/env python3
"""Rustok Wallet install helper bot."""

import os

from telegram import Update
from telegram.ext import Application, CommandHandler, ContextTypes


async def _start_cmd(update: Update, _context: ContextTypes.DEFAULT_TYPE) -> None:
    """Show welcome message."""
    await update.message.reply_text(
        "Welcome to Rustok Wallet! 🦀\n\n"
        "This bot helps you install the local agent wallet.\n"
        "Send /install to get platform-specific instructions."
    )


async def _install_cmd(update: Update, _context: ContextTypes.DEFAULT_TYPE) -> None:
    """Return installation instructions for the user's platform."""
    msg = (
        "Choose your platform:\n\n"
        "<b>Docker (quickest):</b>\n"
        "<pre>"
        "docker run -p 127.0.0.1:3000:3000 \\\n"
        "  -v ~/.rustok/agent:/data \\\n"
        "  -e RUSTOK_AGENT_PASSWORD=\"your-password\" \\\n"
        "  ghcr.io/temrjan/rustok-agent-mcp:latest"
        "</pre>\n\n"
        "<b>Cargo (Rust toolchain):</b>\n"
        "<pre>cargo install rustok-agent-mcp</pre>\n\n"
        "<b>From source:</b>\n"
        "<pre>"
        "git clone https://github.com/temrjan/rustok.git\n"
        "cd rustok\n"
        "cargo run --bin rustok-agent-mcp -- --create-wallet"
        "</pre>\n\n"
        "After install, the wallet API is available at <code>http://127.0.0.1:3000</code>."
    )
    await update.message.reply_text(msg, parse_mode="HTML")


def main() -> None:
    token = os.environ.get("TELEGRAM_BOT_TOKEN")
    if not token:
        raise SystemExit("TELEGRAM_BOT_TOKEN env var required")
    app = Application.builder().token(token).build()
    app.add_handler(CommandHandler("start", _start_cmd))
    app.add_handler(CommandHandler("install", _install_cmd))
    app.run_polling()


if __name__ == "__main__":
    main()
