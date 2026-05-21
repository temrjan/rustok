# OpenClaw Skill Deployment Plan — rustok-wallet

> **Parent:** `docs/AGENT-WALLET-ROADMAP.md` § Phase 4: OpenClaw Skill  
> **Not to be confused with:** `docs/PHASE4-HANDOFF.md` (mobile onboarding flow)  
> **Branch:** `feat/openclaw-skill`  
> **Server:** 7demo (62.169.20.2)  
> **Date:** 2026-05-21  

---

## Goal

OpenClaw agent in Telegram can manage the Agent Wallet via `rustok-agent-mcp` (MCP-over-HTTP). All code changes are local; deployment is `git push → CI → CD` only. No manual SSH edits except one-time bootstrap.

---

## Architecture

```
┌──────────────────┐     curl      ┌──────────────────┐
│ OpenClaw Gateway │ ─────────────→│ rustok-agent-mcp │
│   (Docker)       │               │   (Docker)       │
│   172.22.0.4     │               │   172.22.0.x     │
└──────────────────┘               └──────────────────┘
        ↑                                    │
        │                                    ↓
   Telegram Bot                        ~/.rustok/agent
   (persistent volume)
```

- **Network:** `openclaw_default` (external) — both containers share one Docker bridge network.
- **Data:** Docker volume `rustok-agent-data` — keystore + audit log survive restarts.
- **No SSH edits** — server is bootstrapped once; everything else via GitHub Actions.

---

## Local Code Changes (before push)

| File | Change |
|------|--------|
| `crates/agent-mcp/src/main.rs` | Add `--host <HOST>` CLI arg (default `127.0.0.1`) |
| `crates/agent-mcp/src/server.rs` | `McpServer::run(self, host: &str, port: u16)` — bind to `{host}:{port}` |
| `skills/rustok-wallet/SKILL.md` | Single-line JSON frontmatter (`metadata.openclaw`), inline `curl` commands, hardcoded `rustok-agent-mcp:3000` |
| `crates/agent-mcp/Dockerfile` | Multi-stage: `rust:1.85-slim-bookworm` builder → `debian:bookworm-slim` runtime with `ca-certificates` + `curl` |
| `.dockerignore` | Exclude `target/`, `.git/`, `node_modules/`, `mobile/`, `app/src/`, `docs/` |
| `docker-compose.yml` | Service `rustok-agent-mcp`: `image: ghcr.io/...`, volume `rustok-agent-data:/root/.rustok/agent`, network `openclaw_default` (external), healthcheck via `curl /health` |
| `.github/workflows/deploy-agent-mcp.yml` | CD: `paths` filter → build & push `ghcr.io/...` → SSH to 7demo: `git pull`, `docker compose pull && up -d` |
| `.github/workflows/ci.yml` | Update `skill` job: validate `name` + `description` + optional `metadata.openclaw` |

---

## Bootstrap on 7demo (one-time manual steps)

> ⚠️ These are the only allowed manual steps per Codex pipeline standards.

```bash
# 1. Ensure repo is cloned
mkdir -p /root/rustok && cd /root/rustok
git clone https://github.com/temrjan/rustok.git . || git pull origin main

# 2. Create wallet data directory and .env for Docker Compose
mkdir -p /root/.rustok/agent
echo "RUSTOK_AGENT_PASSWORD=<strong_password>" > /root/server/.env

# 3. First deploy only: build image locally (registry is empty)
#    Subsequent deploys use `docker compose pull` from CI/CD
cd /root/server
docker compose up -d --build rustok-agent-mcp

# 4. One-shot wallet creation (uses running service image)
docker compose run --rm rustok-agent-mcp \
  --host 0.0.0.0 --port 3000 \
  --data-dir /root/.rustok/agent \
  --unlock-env --create-wallet

# 5. Restart persistent service (after wallet exists, remove --create-wallet from commands)
docker compose up -d rustok-agent-mcp

# 5. Configure OpenClaw skill entry in openclaw.json
# Add to /root/.openclaw/openclaw.json under "skills.entries":
#   "rustok-wallet": {
#     "enabled": true,
#     "env": { "RUSTOK_AGENT_PASSWORD": "<same_password>" }
#   }

# 6. Publish skill to ClawHub (from local machine)
#   npm install -g clawhub
#   clawhub login
#   clawhub publish ./skills/rustok-wallet

# 7. Install skill on 7demo (inside OpenClaw container or via CLI)
#   openclaw skills install rustok-wallet --global
```

---

## CD Pipeline

### Trigger
```yaml
on:
  push:
    branches: [main]
    paths:
      - "crates/agent-mcp/**"
      - "crates/agent-wallet/**"
      - "crates/agent-dapps/**"
      - "crates/core/**"
      - "docker-compose.yml"
      - ".github/workflows/deploy-agent-mcp.yml"
```

### Jobs
1. **build-and-push** — Docker Buildx → `ghcr.io/temrjan/rustok-agent-mcp:latest`
2. **deploy** — SSH to 7demo: `git pull`, `docker compose pull && up -d`, verify health

### Secrets required in GitHub
| Secret | Value |
|--------|-------|
| `DEPLOY_HOST` | `62.169.20.2` |
| `DEPLOY_USER` | `root` |
| `DEPLOY_SSH_KEY` | Private SSH key for 7demo |
| `DEPLOY_PORT` | `9281` |

---

## Gates (run locally before push)

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## Verify after deploy

```bash
# From inside OpenClaw container
docker exec openclaw-openclaw-gateway-1 \
  curl -fsS http://rustok-agent-mcp:3000/health

# Telegram test: "show my wallet context" → agent returns address + balances
```

---

## Security Notes

- `RUSTOK_AGENT_PASSWORD` is injected via `skills.entries.rustok-wallet.env` in `openclaw.json` — scoped to the agent run, not stored in shell environment.
- Wallet data is isolated in Docker volume `rustok-agent-data`.
- Policy limits are code-level; LLM cannot bypass them.
- `--create-wallet` is used only in the one-shot bootstrap command, not in persistent `docker-compose.yml`.

---

## References

- `docs/AGENT-WALLET-ROADMAP.md` — parent roadmap
- `docs/AGENT-WALLET-REVIEW-HANDOFF.md` — review handoff
- `skills/rustok-wallet/SKILL.md` — skill definition
- `crates/agent-mcp/` — MCP server source
