# Rustok RPC proxy

A minimal Cloudflare Worker that proxies Ethereum JSON-RPC and Blockscout API
requests through a single stable TLS endpoint.

## Why

Android `rustls-platform-verifier` performs strict OCSP revocation checks.
Many public RPC and explorer endpoints use Let's Encrypt certificates, which
no longer publish OCSP URLs (Let's Encrypt retired OCSP in August 2025). The
platform verifier interprets a missing OCSP URL as `Revoked` and fails the
TLS handshake. The failure surfaces to users as "chains failed" on the Home
screen and "chains unavailable" on the Activity screen.

The Cloudflare edge serves one cert per hostname (including OCSP), so routing
client traffic through `rpc.rustokwallet.com` removes the whole failure class.
Upstream calls are server-to-server, where Android is not involved.

## Endpoints

| Route | Method | Target |
|-------|--------|--------|
| `POST /rpc/{chain}` | JSON-RPC | `publicnode` → `drpc` / `cloudflare-eth` |
| `GET /explorer/{chain}/api?...` | REST | `blockscout.com` |
| `GET /health` | — | liveness probe |
| `GET /` | — | usage payload |

`{chain}` is one of `ethereum`, `arbitrum`, `base`, `optimism`, `zksync`, `sepolia`, `arbitrum-sepolia`.

## Local development

```bash
cd deploy/rpc-proxy
npm install
npm run dev               # http://localhost:8787
```

Smoke test:

```bash
curl -s -X POST http://localhost:8787/rpc/ethereum \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}'
# -> {"jsonrpc":"2.0","result":"0x1","id":1}
```

## Deployment

Requires Cloudflare account (zone `rustokwallet.com`).

```bash
cd deploy/rpc-proxy
npm install
npx wrangler login        # one-time
npx wrangler deploy       # publishes to <project>.workers.dev
```

After the first deploy, bind the custom domain manually (one-time):

1. Cloudflare Dashboard → Workers & Pages → `rustok-rpc-proxy` → Settings →
   Triggers → Custom Domains → **Add Custom Domain**
2. Enter `rpc.rustokwallet.com` and confirm
3. Cloudflare provisions the DNS record and issues a TLS cert automatically

Subsequent deploys are just `npx wrangler deploy`.

## Observability

`wrangler tail` streams live logs.

## Rate limiting

Handled in Cloudflare Dashboard → zone `rustokwallet.com` → Security → WAF →
Rate limiting rules. Recommend 100 req/min per IP to `rpc.rustokwallet.com`.
