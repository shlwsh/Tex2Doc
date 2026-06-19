"""Verify DeepSeek Chat Completions API connectivity.

Mirrors the original curl:
  POST https://api.deepseek.com/chat/completions
  Authorization: Bearer <sk-...>
  model = deepseek-v4-pro  (thinking enabled, reasoning_effort=high)

Run:
  python verify_deepseek.py
"""

import json
import os
import sys
import urllib.error
import urllib.request

ENDPOINT = "https://api.deepseek.com/chat/completions"
TOKEN = "sk-REDACTED-DEEPSEEK"
MODEL = "deepseek-v4-pro"


def call() -> dict:
    body = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": "用一句话介绍你自己，并证明连接正常。"},
        ],
        "thinking": {"type": "enabled"},
        "reasoning_effort": "high",
        "stream": False,
    }
    req = urllib.request.Request(
        ENDPOINT,
        data=json.dumps(body).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {TOKEN}",
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.loads(r.read().decode("utf-8"))


def main() -> int:
    try:
        data = call()
    except urllib.error.HTTPError as e:
        print(f"[FAIL] HTTP {e.code}: {e.read().decode('utf-8', 'replace')}")
        return 1
    except Exception as e:
        print(f"[FAIL] {type(e).__name__}: {e}")
        return 2

    if "choices" not in data or not data["choices"]:
        print(f"[FAIL] unexpected payload: {json.dumps(data)[:300]}")
        return 3

    msg = data["choices"][0].get("message", {})
    content = (msg.get("content") or "").strip()
    reasoning = (msg.get("reasoning_content") or "").strip()

    print(f"[OK] model={data.get('model')}  "
          f"finish_reason={data['choices'][0].get('finish_reason')}")
    print(f"     usage={data.get('usage')}")
    print("----- reasoning -----")
    print(reasoning[:400] + ("..." if len(reasoning) > 400 else ""))
    print("----- reply -----")
    print(content)
    print("-----------------")
    return 0


if __name__ == "__main__":
    sys.exit(main())
