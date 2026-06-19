"""Verify the modified mygit.py AI-summary path end-to-end with DeepSeek.

- Loads `.env.mygit` exactly like the real script does.
- Calls the same code path (call_dashscope_api → DeepSeek) to generate a
  commit message for a real-style diff.
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "scripts"))

import mygit  # noqa: E402

# 用项目根作为 workspace，但只读 .env.mygit，不触碰 git
WORKSPACE = os.path.dirname(os.path.abspath(__file__))
SCRIPT_DIR = os.path.join(WORKSPACE, "scripts")

config = mygit.load_config(WORKSPACE, SCRIPT_DIR)

# 复用 main() 里的读取顺序（已合并到 mygit.py）
api_key = (
    config.get("DEEPSEEK_API_KEY")
    or config.get("DASHSCOPE_API_KEY")
    or ""
).strip()
base_url = (
    config.get("DEEPSEEK_BASE_URL")
    or config.get("DASHSCOPE_BASE_URL")
    or "https://api.deepseek.com/v1"
).rstrip("/")
model = (
    config.get("DEEPSEEK_MODEL")
    or config.get("DASHSCOPE_MODEL")
    or "deepseek-v4-pro"
).strip()

print(f"[cfg] api_key=sk-...{api_key[-6:] if len(api_key) > 6 else '(short)'}")
print(f"[cfg] base_url={base_url}")
print(f"[cfg] model={model}")
print()

# 真实风格 diff：把上一轮 Claude 网关 + DeepSeek 验证的几步改动喂给 AI
status_output = (
    " M scripts/mygit.py\n"
    "?? verify_claude_max.py\n"
    "?? verify_deepseek.py\n"
    "?? probe_gateway.py\n"
    "?? simulate_mygit_ai.py\n"
)
stat = (
    " scripts/mygit.py | 22 ++++++++++++----------\n"
    " 5 files changed, 12 insertions(+), 10 deletions(-)\n"
)
diff = (
    "diff --git a/scripts/mygit.py b/scripts/mygit.py\n"
    "@@ -820,7 +820,16 @@\n"
    "-    api_key = config.get(\"DASHSCOPE_API_KEY\", \"\").strip()\n"
    "-    base_url = config.get(\"DASHSCOPE_BASE_URL\", \"\").rstrip(\"/\")\n"
    "-    model = config.get(\"DASHSCOPE_MODEL\", \"\").strip()\n"
    "+    api_key = (\n"
    "+        config.get(\"DEEPSEEK_API_KEY\")\n"
    "+        or config.get(\"DASHSCOPE_API_KEY\")\n"
    "+        or \"\"\n"
    "+    ).strip()\n"
    "+    base_url = (\n"
    "+        config.get(\"DEEPSEEK_BASE_URL\")\n"
    "+        or config.get(\"DASHSCOPE_BASE_URL\")\n"
    "+        or \"https://api.deepseek.com/v1\"\n"
    "+    ).rstrip(\"/\")\n"
    "+    model = (\n"
    "+        config.get(\"DEEPSEEK_MODEL\")\n"
    "+        or config.get(\"DASHSCOPE_MODEL\")\n"
    "+        or \"deepseek-v4-pro\"\n"
    "+    ).strip()\n"
)
user_prompt = (
    f"变更摘要:\n{status_output}\n\n"
    f"变更详情:\n{mygit.build_staged_diff(['scripts/mygit.py'], [], stat, diff)}"
)
system_prompt = mygit.P3_SYSTEM_PROMPT.replace("p3-microservice", "Tex2Doc")

import requests  # noqa: E402

session = requests.Session()
payload = {
    "model": model,
    "messages": [
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": user_prompt},
    ],
    "max_tokens": 500,
    "temperature": 0.7,
}
headers = {
    "Content-Type": "application/json",
    "Authorization": f"Bearer {api_key}",
}
proxy_url = mygit.resolve_proxy(config)
print(f"[net] proxy={proxy_url or '(direct)'}")

resp = mygit.call_dashscope_api(
    session, f"{base_url}/chat/completions", headers, payload, proxy_url
)
commit_msg = mygit.strip_markdown_fence(
    resp.json()["choices"][0]["message"]["content"]
)

print("\n----- AI 生成提交信息 -----")
print(commit_msg)
print("----------------------------")
print(f"usage={resp.json().get('usage')}")
