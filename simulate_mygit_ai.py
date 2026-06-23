"""Simulate mygit.py AI-summary path against DeepSeek, mirroring the real call.

Reuses the exact system_prompt and payload shape from scripts/mygit.py
so we know it works inside mygit's flow.
"""

import json
import os
import urllib.request

ENDPOINT = "https://api.deepseek.com/chat/completions"
TOKEN = "sk-REDACTED-DEEPSEEK"
MODEL = "deepseek-v4-pro"

# 复制自 mygit.py P3_SYSTEM_PROMPT（仅替换项目名为 Tex2Doc）
SYSTEM_PROMPT = (
    "你是一个专业的 Git 提交信息生成助手，熟悉 Tex2Doc 项目："
    "分布式定向日志采集组件（Go Agent/Center、gRPC、Loki、Redis、"
    "OpenResty 网关、Docker Compose 部署、科研实验脚本与 LaTeX 论文）。"
    "请根据代码变更生成简洁、清晰的中文提交信息。\n"
    "规范：第一行使用 Conventional Commits 前缀（feat/fix/docs/chore/refactor/test 等），"
    "可加 scope（agent/center/deploy/experiments/docs/latex/scripts/proto）；"
    "标题不超过 50 字；使用中文；不要 Markdown 代码块或多余解释。"
)

USER_PROMPT = """变更摘要:
 M scripts/mygit.py
?? verify_claude_max.py
?? verify_deepseek.py
?? probe_gateway.py

变更详情:
 scripts/mygit.py | 6 +++---
 1 file changed, 3 insertions(+), 3 deletions(-)

diff --git a/scripts/mygit.py b/scripts/mygit.py
@@ -820,7 +820,7 @@
-    api_key = config.get("DASHSCOPE_API_KEY", "").strip()
-    base_url = config.get("DASHSCOPE_BASE_URL", "").rstrip("/")
-    model = config.get("DASHSCOPE_MODEL", "").strip()
+    api_key = (config.get("DASHSCOPE_API_KEY") or config.get("DEEPSEEK_API_KEY") or "").strip()
+    base_url = (config.get("DASHSCOPE_BASE_URL") or config.get("DEEPSEEK_BASE_URL") or "https://api.deepseek.com/v1").rstrip("/")
+    model = (config.get("DASHSCOPE_MODEL") or config.get("DEEPSEEK_MODEL") or "deepseek-v4-pro").strip()
"""


def main() -> None:
    payload = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": USER_PROMPT},
        ],
        "max_tokens": 500,
        "temperature": 0.7,
        "stream": False,
    }
    req = urllib.request.Request(
        ENDPOINT,
        data=json.dumps(payload).encode("utf-8"),
        headers={
            "Content-Type": "application/json",
            "Authorization": f"Bearer {TOKEN}",
        },
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=60) as r:
        data = json.loads(r.read().decode("utf-8"))

    content = data["choices"][0]["message"]["content"].strip()
    # mygit.py 走 strip_markdown_fence；这里手动做同样处理
    if content.startswith("```"):
        lines = content.split("\n")
        if lines[0].startswith("```"):
            lines = lines[1:]
        if lines and lines[-1].strip() == "```":
            lines = lines[:-1]
        content = "\n".join(lines).strip()

    print("----- AI 生成提交信息 -----")
    print(content)
    print("----------------------------")
    print(f"usage={data.get('usage')}")


if __name__ == "__main__":
    main()
