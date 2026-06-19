"""Probe the gateway with different header sets to find which it accepts."""

import json
import os
import sys
import urllib.request

ENDPOINT = "https://api.ymhss.cn/claude-max/v1/messages"
TOKEN = "sk-ccmax-c8bc6b4615551a555ef078a7123cdb9f481d7feffbabb2f1"
MODEL = "claude-3-5-sonnet-latest"

body = json.dumps({
    "model": MODEL,
    "max_tokens": 64,
    "messages": [{"role": "user", "content": "ping"}],
}).encode()

CASES = [
    ("default-anthropic-sdk", {
        "content-type": "application/json",
        "x-api-key": TOKEN,
        "anthropic-version": "2023-06-01",
    }),
    ("claude-code-ua", {
        "content-type": "application/json",
        "x-api-key": TOKEN,
        "anthropic-version": "2023-06-01",
        "user-agent": "claude-code/1.0.0",
    }),
    ("claude-code-full", {
        "content-type": "application/json",
        "x-api-key": TOKEN,
        "anthropic-version": "2023-06-01",
        "user-agent": "claude-code/1.0.0",
        "anthropic-dangerous-direct-browser-access": "true",
    }),
    ("auth-token-bearer", {
        "content-type": "application/json",
        "authorization": f"Bearer {TOKEN}",
        "anthropic-version": "2023-06-01",
        "user-agent": "claude-code/1.0.0",
    }),
]

for name, headers in CASES:
    req = urllib.request.Request(ENDPOINT, data=body, headers=headers, method="POST")
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            data = r.read().decode("utf-8", "replace")[:200]
            print(f"[{name}] HTTP {r.status} -> {data}")
    except urllib.error.HTTPError as e:
        msg = e.read().decode("utf-8", "replace")[:200]
        print(f"[{name}] HTTP {e.code} -> {msg}")
    except Exception as e:
        print(f"[{name}] ERR {type(e).__name__}: {e}")
