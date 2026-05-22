#!/usr/bin/env python3
"""MCP stdio wrapper for rustok-agent-mcp HTTP API."""
import json, sys, urllib.request, urllib.error

BASE = "http://127.0.0.1:3000"

def call(method, path, body=None):
    url = BASE + path
    data = json.dumps(body).encode() if body else None
    req = urllib.request.Request(url, data=data, headers={"Content-Type": "application/json"}, method=method)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        return {"error": e.read().decode(), "status": e.code}
    except Exception as e:
        return {"error": str(e)}

def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        msg = json.loads(line)
        if msg.get("method") == "initialize":
            print(json.dumps({"jsonrpc": "2.0", "id": msg.get("id"), "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "serverInfo": {"name": "rustok-wallet", "version": "0.1.0"}
            }}), flush=True)
        elif msg.get("method") == "tools/list":
            tools = [
                {"name": "wallet_context", "description": "Get wallet context (balances, limits, gas, positions)", "inputSchema": {"type": "object", "properties": {}, "required": []}},
                {"name": "wallet_positions", "description": "Get DeFi positions", "inputSchema": {"type": "object", "properties": {"address": {"type": "string"}}, "required": []}},
                {"name": "preview_transaction", "description": "Preview ETH send", "inputSchema": {"type": "object", "properties": {"to": {"type": "string"}, "amount_wei": {"type": "string"}, "chain_id": {"type": "integer"}}, "required": ["to", "amount_wei", "chain_id"]}},
                {"name": "execute_transaction", "description": "Execute ETH send", "inputSchema": {"type": "object", "properties": {"to": {"type": "string"}, "amount_wei": {"type": "string"}, "chain_id": {"type": "integer"}, "preview_id": {"type": "string"}}, "required": ["to", "amount_wei", "chain_id", "preview_id"]}},
            ]
            print(json.dumps({"jsonrpc": "2.0", "id": msg.get("id"), "result": {"tools": tools}}), flush=True)
        elif msg.get("method") == "tools/call":
            params = msg.get("params", {})
            name = params.get("name")
            args = params.get("arguments", {})
            if name == "wallet_context":
                result = call("POST", "/context")
            elif name == "wallet_positions":
                result = call("POST", "/positions", {"address": args.get("address")})
            elif name == "preview_transaction":
                result = call("POST", "/preview", {"to": args["to"], "amount_wei": args["amount_wei"], "chain_id": args["chain_id"]})
            elif name == "execute_transaction":
                result = call("POST", "/execute", {"to": args["to"], "amount_wei": args["amount_wei"], "chain_id": args["chain_id"], "preview_id": args["preview_id"]})
            else:
                result = {"error": "unknown tool"}
            content = [{"type": "text", "text": json.dumps(result, indent=2)}]
            print(json.dumps({"jsonrpc": "2.0", "id": msg.get("id"), "result": {"content": content, "isError": "error" in result}}), flush=True)
        else:
            print(json.dumps({"jsonrpc": "2.0", "id": msg.get("id"), "error": {"code": -32601, "message": "Method not found"}}), flush=True)

if __name__ == "__main__":
    main()
