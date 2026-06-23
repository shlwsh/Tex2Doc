"""Verify connectivity to ANTHROPIC_BASE_URL via ANTHROPIC_AUTH_TOKEN.

Usage:
  pip install anthropic
  python verify_claude_max.py
"""

import os
import sys

from anthropic import Anthropic

ENDPOINT = "https://api.ymhss.cn/claude-max"
TOKEN = "sk-ccmax-c8bc6b4615551a555ef078a7123cdb9f481d7feffbabb2f1"
MODEL = "claude-3-5-sonnet-latest"


def main() -> int:
    os.environ.setdefault("ANTHROPIC_BASE_URL", ENDPOINT)
    os.environ.setdefault("ANTHROPIC_AUTH_TOKEN", TOKEN)

    client = Anthropic(base_url=ENDPOINT, auth_token=TOKEN)

    try:
        resp = client.messages.create(
            model=MODEL,
            max_tokens=128,
            messages=[
                {"role": "user", "content": "用一句话介绍你自己，并证明连接正常。"},
            ],
        )
    except Exception as e:
        print(f"[FAIL] call error: {type(e).__name__}: {e}")
        return 1

    text = "".join(
        b.text for b in resp.content if getattr(b, "type", "") == "text"
    ).strip()

    if not text:
        print("[FAIL] empty response")
        return 2

    print(f"[OK] model={resp.model}  stop_reason={resp.stop_reason}")
    usage = resp.usage.model_dump() if resp.usage else None
    print(f"     usage={usage}")
    print("----- reply -----")
    print(text)
    print("-----------------")
    return 0


if __name__ == "__main__":
    sys.exit(main())
