#!/usr/bin/env python3
"""AI Git 提交工具：自动 add → commit → push（适配 p3-microservice / WSL2）"""

from __future__ import annotations

import os
import re
import socket
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime
from typing import Iterable

import requests

WIN_GIT = "/mnt/c/Program Files/Git/cmd/git.exe"
GCM_WRAPPER = os.path.join(os.path.dirname(__file__), "git-credential-gcm.sh")
PROXY_PORTS = ("7897", "7890", "10809", "1080")

# 自动提交时永不纳入版本库
AUTO_COMMIT_NEVER_FILES = (".env", ".env.local")

# 构建/索引产物，不自动暂存
AUTO_COMMIT_EXCLUDE_PREFIXES = (
    ".gitnexus/",
    "experiments/results/tmp/",
    "__pycache__/",
    "latex/output/",
)

# LaTeX / 编译中间文件（不参与 diff 文本）
LATEX_AUX_SUFFIXES = (".log", ".aux", ".bbl", ".blg", ".out", ".toc", ".fls", ".fdb_latexmk")

# 版本/发布相关文件（变更时提示确认）
VERSION_FILES = (
    "agent/go.mod",
    "center/go.mod",
    "proto/go.mod",
    "deploy/docker/docker-compose.yml",
    "deploy/docker/docker-compose.wsl.yml",
)

BINARY_SUFFIXES = (
    ".pdf",
    ".zip",
    ".png",
    ".jpg",
    ".jpeg",
    ".gif",
    ".webp",
    ".svg",
    ".ico",
    ".bin",
    ".so",
    ".o",
    ".deb",
    ".exe",
    ".dll",
    ".pt",
    ".onnx",
    ".pth",
    ".ckpt",
)

PLACEHOLDER_API_KEYS = {
    "",
    "your-api-key-here",
    "sk-your-api-key",
    "sk-你的密钥",
    "changeme",
}

P3_SYSTEM_PROMPT = (
    "你是一个专业的 Git 提交信息生成助手，熟悉 p3-microservice 项目："
    "分布式定向日志采集组件（Go Agent/Center、gRPC、Loki、Redis、"
    "OpenResty 网关、Docker Compose 部署、科研实验脚本与 LaTeX 论文）。"
    "请根据代码变更生成简洁、清晰的中文提交信息。\n"
    "规范：第一行使用 Conventional Commits 前缀（feat/fix/docs/chore/refactor/test 等），"
    "可加 scope（agent/center/deploy/experiments/docs/latex/scripts/proto）；"
    "标题不超过 50 字；使用中文；不要 Markdown 代码块或多余解释。"
)


@dataclass
class ChangeStatus:
    modified: list[str]
    added: list[str]
    deleted: list[str]
    untracked: list[str]
    excluded: list[str]

    @property
    def all_files(self) -> list[str]:
        return self.modified + self.added + self.deleted + self.untracked

    @property
    def has_changes(self) -> bool:
        return bool(self.all_files)


def run_command(command: str, check: bool = True, env: dict | None = None) -> str | None:
    result = subprocess.run(
        command,
        shell=True,
        capture_output=True,
        env=env,
    )
    if check and result.returncode != 0:
        return None
    if not result.stdout:
        return ""
    return result.stdout.decode("utf-8", errors="replace").strip()


def load_env_file(path: str) -> dict[str, str]:
    config: dict[str, str] = {}
    if not os.path.exists(path):
        return config
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#") or "=" not in line:
                continue
            key, value = line.split("=", 1)
            config[key.strip()] = value.strip().strip("'").strip('"')
    return config


def load_config(workspace: str, script_dir: str) -> dict[str, str]:
    global_env = os.path.join(script_dir, "..", ".env.mygit")
    local_env = os.path.join(workspace, ".env.mygit")
    
    config = {}
    if os.path.exists(global_env):
        config.update(load_env_file(global_env))
    if os.path.exists(local_env):
        config.update(load_env_file(local_env))
        
    for extra in (".env", ".env.local"):
        extra_path = os.path.join(workspace, extra)
        if os.path.exists(extra_path):
            for key, value in load_env_file(extra_path).items():
                if key in ("GITHUB_TOKEN", "GH_TOKEN") or key not in config:
                    config[key] = value
    return config


def is_port_open(host: str, port: int | str, timeout: float = 0.8) -> bool:
    try:
        with socket.create_connection((host, int(port)), timeout=timeout):
            return True
    except OSError:
        return False


def resolve_proxy(config: dict[str, str]) -> str | None:
    explicit = (
        config.get("MYGIT_HTTP_PROXY")
        or config.get("https_proxy")
        or config.get("HTTPS_PROXY")
        or config.get("http_proxy")
        or config.get("HTTP_PROXY")
        or os.environ.get("MYGIT_HTTP_PROXY")
        or os.environ.get("https_proxy")
        or os.environ.get("HTTPS_PROXY")
    )
    if explicit:
        explicit = explicit.rstrip("/")
        if "://" not in explicit:
            explicit = f"http://{explicit}"
        body = explicit.split("://", 1)[1]
        host = body.split(":")[0].split("/")[0]
        port = 7897
        if ":" in body.split("/")[0]:
            port = int(body.split(":")[1].split("/")[0])
        if is_port_open(host, port):
            return f"http://{host}:{port}"

    for port in PROXY_PORTS:
        if is_port_open("127.0.0.1", port):
            return f"http://127.0.0.1:{port}"

    try:
        with open("/etc/resolv.conf", encoding="utf-8") as f:
            for line in f:
                if line.startswith("nameserver"):
                    host = line.split()[1]
                    if host.startswith("198.18."):
                        continue
                    for port in PROXY_PORTS:
                        if is_port_open(host, port):
                            return f"http://{host}:{port}"
    except OSError:
        pass
    return None


def apply_proxy_env(env: dict, proxy_url: str | None) -> dict:
    if not proxy_url:
        return env
    out = env.copy()
    for key in (
        "http_proxy",
        "https_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "all_proxy",
        "ALL_PROXY",
    ):
        out[key] = proxy_url
    return out


def clean_env_for_windows_git() -> dict[str, str]:
    skip_parts = ("proxy", "PROXY", "SSL", "CURL", "GIT_SSL", "GIT_HTTP")
    return {
        key: value
        for key, value in os.environ.items()
        if not any(part in key for part in skip_parts)
    }


def is_binary_artifact(file_path: str) -> bool:
    lower = file_path.replace("\\", "/").lower()
    return lower.endswith(BINARY_SUFFIXES) or lower.endswith(LATEX_AUX_SUFFIXES)


def is_wsl_linux() -> bool:
    try:
        with open("/proc/version", encoding="utf-8") as f:
            return "microsoft" in f.read().lower()
    except OSError:
        return False


def win_git_available() -> bool:
    if not os.path.isfile(WIN_GIT):
        return False
    try:
        probe = subprocess.run(
            [WIN_GIT, "--version"],
            capture_output=True,
            timeout=5,
        )
        return probe.returncode == 0
    except (OSError, subprocess.TimeoutExpired):
        return False


def is_excluded_from_auto_commit(file_path: str) -> bool:
    normalized = file_path.replace("\\", "/")
    if normalized in AUTO_COMMIT_NEVER_FILES or normalized.endswith(
        tuple(f"/{f}" for f in AUTO_COMMIT_NEVER_FILES)
    ):
        return True
    if normalized.startswith(".env copy") or "/.env copy" in normalized:
        return True
    if normalized.endswith(".pyc") or "/__pycache__/" in normalized:
        return True
    return any(normalized.startswith(prefix) for prefix in AUTO_COMMIT_EXCLUDE_PREFIXES)


def parse_porcelain_path(line: str) -> tuple[str, str]:
    """解析 git status --porcelain 行，返回 (status, path)。"""
    if len(line) < 3:
        return "", ""
    status = line[:2]
    file_path = line[2:].lstrip()
    if " -> " in file_path:
        file_path = file_path.split(" -> ", 1)[1].strip()
    return status, file_path


def parse_git_status(output: str) -> ChangeStatus:
    modified: list[str] = []
    added: list[str] = []
    deleted: list[str] = []
    untracked: list[str] = []
    excluded: list[str] = []

    for line in output.splitlines():
        if not line:
            continue
        status, file_path = parse_porcelain_path(line)
        if not file_path:
            continue
        if is_excluded_from_auto_commit(file_path):
            excluded.append(file_path)
            continue
        if "M" in status:
            modified.append(file_path)
        elif "A" in status:
            added.append(file_path)
        elif "D" in status:
            deleted.append(file_path)
        elif status == "??":
            untracked.append(file_path)

    return ChangeStatus(modified, added, deleted, untracked, excluded)


def format_status_line(status: str, file_path: str) -> str:
    if status in {"M", "AM", "MM"} or "M" in status:
        return f"  修改: {file_path}"
    if status == "A" or status.startswith("A"):
        return f"  新增: {file_path}"
    if "D" in status:
        return f"  删除: {file_path}"
    if status == "??":
        return f"  未跟踪: {file_path}"
    return f"  其他: {file_path}"


def call_dashscope_api(
    session: requests.Session,
    url: str,
    headers: dict,
    payload: dict,
    proxy_url: str | None,
) -> requests.Response:
    attempts: list[dict | None] = []
    if proxy_url:
        attempts.append({"http": proxy_url, "https": proxy_url})
    attempts.append({"http": None, "https": None})

    last_error: Exception | None = None
    for proxies in attempts:
        try:
            resp = session.post(url, headers=headers, json=payload, timeout=60, proxies=proxies)
            resp.raise_for_status()
            return resp
        except (requests.RequestException, OSError) as exc:
            last_error = exc
    raise last_error or RuntimeError("AI 请求失败")


# =============================================================================
# 本地 Ollama（OpenAI 兼容端点）支持
# =============================================================================
# Ollama 暴露 /v1/chat/completions（OpenAI 兼容），可零改造复用调用逻辑。
# 与 DashScope 的区别：
#   1) 端点固定为本机 127.0.0.1:11434，不走代理；
#   2) Authorization 可省略（Ollama 默认本地无鉴权），脚本即便配置了也忽略；
#   3) 默认模型 gemma4:e4b（4-bit 量化、约 8B，CPU/GPU 都能跑），需在 Ollama
#      中 `ollama pull gemma4:e4b` 才会存在。
# 通过 MYGIT_PREFER_OLLAMA=0 关闭本节逻辑，回到纯 DashScope 流程。
OLLAMA_DEFAULT_BASE_URL = "http://127.0.0.1:11434/v1"
OLLAMA_DEFAULT_MODEL = "gemma4:e4b"
OLLAMA_PROBE_TIMEOUT = 1.5  # 探测阶段超时（秒）


def _strip_v1_suffix(url: str) -> str:
    """去掉末尾的 /v1，便于拼 /models、/chat/completions。"""
    return url.rstrip("/").removesuffix("/v1")


def detect_ollama(config: dict[str, str]) -> tuple[str, str] | None:
    """探测本地 Ollama 服务是否在线且指定模型已下载。

    Returns:
        (base_url, model) —— 可用于调用；None 表示不可用。
    """
    if config.get("MYGIT_PREFER_OLLAMA") == "0":
        return None

    base_raw = (config.get("OLLAMA_BASE_URL") or OLLAMA_DEFAULT_BASE_URL).strip()
    model = (config.get("OLLAMA_MODEL") or OLLAMA_DEFAULT_MODEL).strip()
    base = _strip_v1_suffix(base_raw)
    # 探测：GET /api/tags（无 model 参数，跨 Ollama 版本稳定），
    # 再从返回的 models 列表里查找目标 model（兼容 ":latest" 后缀）。
    try:
        resp = requests.get(
            f"{base}/api/tags",
            timeout=OLLAMA_PROBE_TIMEOUT,
        )
        if resp.status_code != 200:
            return None
        names = {m.get("name", "") for m in (resp.json().get("models") or [])}
        if model in names or f"{model}:latest" in names:
            return f"{base}/v1", model
        # 宽松匹配：仅按短名（gemma4:e4b → gemma4）查找
        short = model.split(":")[0]
        if any(n.split(":")[0] == short for n in names if n):
            return f"{base}/v1", model
        return None
    except (requests.RequestException, OSError, ValueError):
        return None


def call_ollama_api(
    session: requests.Session,
    url: str,
    payload: dict,
) -> requests.Response:
    """调用本地 Ollama。

    注意：Gemma 4 在 /v1/chat/completions（OpenAI 兼容）端点上会强制消耗
    所有 max_tokens 于内部 thinking 过程，导致 content 为空字符串（Ollama
    OpenAI 兼容层不接受 think=False 参数）。本函数改走 Ollama 原生
    /api/chat 端点并设置 think=False，从而让本地模型真正输出 commit 信息。

    本机调用不走代理；超时放宽到 300s（首次加载模型约 10-30s，
    在 8B Q4 模型 + 大 diff 场景下前向推理可能再吃 30-120s）。
    """
    # 把 OpenAI 风格的 url 改写到原生 /api/chat
    native_url = url.replace("/v1/chat/completions", "/api/chat")
    native_payload = {
        "model": payload["model"],
        "messages": payload["messages"],
        "stream": False,
        "think": False,  # 关键:关闭 Gemma 4 thinking,让所有 token 用于实际输出
        "options": {
            "num_ctx": int(payload.get("num_ctx", 8192)),
        },
    }
    if "temperature" in payload:
        native_payload["options"]["temperature"] = payload["temperature"]
    if "max_tokens" in payload:
        native_payload["options"]["num_predict"] = payload["max_tokens"]

    try:
        resp = session.post(
            native_url,
            json=native_payload,
            timeout=300,
            proxies={"http": None, "https": None},
        )
        resp.raise_for_status()
    except requests.RequestException as exc:
        raise RuntimeError(f"Ollama 请求失败: {exc}") from exc

    # 把原生响应包装成 OpenAI 兼容的 choices 形式，让上游 strip_markdown_fence
    # 逻辑无需改动
    data = resp.json()
    message = data.get("message") or {}
    class _Shim:
        def __init__(self, d):
            self._d = d
        def json(self):
            return {
                "choices": [
                    {
                        "message": {
                            "role": message.get("role", "assistant"),
                            "content": message.get("content", ""),
                        }
                    }
                ]
            }
        def raise_for_status(self):
            return None
    return _Shim(data)


def warmup_ollama(base_url: str, model: str) -> bool:
    """同步预热 Ollama：把模型从磁盘加载进内存/显存，返回是否成功。

    8B Q4 模型首次加载常需 10-30s，提前同步预热可避免下一次
    chat/completions 把时间花在前向推理前的模型加载阶段，从而让主调用
    在统一超时内完成。仅在本地调用环境下使用，不影响 DashScope 后端。
    """
    try:
        session = requests.Session()
        payload = {
            "model": model,
            "messages": [{"role": "user", "content": "ok"}],
            "max_tokens": 4,
            "temperature": 0.0,
            "stream": False,
        }
        session.post(
            f"{base_url}/chat/completions",
            json=payload,
            timeout=300,
            proxies={"http": None, "https": None},
        ).raise_for_status()
        return True
    except Exception as exc:
        print(f"⚠️  Ollama 预热失败（{exc}），继续提交流程...")
        return False


def strip_markdown_fence(text: str) -> str:
    message = text.strip()
    if message.startswith("```"):
        lines = message.split("\n")
        if lines[0].startswith("```"):
            lines = lines[1:]
        if lines and lines[-1].strip() == "```":
            lines = lines[:-1]
        message = "\n".join(lines).strip()
    return message


def infer_commit_type(files: Iterable[str]) -> str:
    joined = " ".join(files).lower()
    if re.search(r"\.(md|tex)$|(^|/)docs/", joined):
        return "docs"
    if re.search(r"(^|/)experiments/|test_", joined):
        return "test" if "test" in joined else "feat"
    if re.search(r"fix|bug|hotfix", joined):
        return "fix"
    if re.search(r"\.go$|\.proto$", joined):
        return "feat"
    if re.search(r"go\.mod|docker-compose|Dockerfile|\.ya?ml$", joined):
        return "chore"
    if joined.startswith("scripts/"):
        return "chore"
    return "chore"


def infer_scope(files: Iterable[str]) -> str | None:
    scopes: set[str] = set()
    for file_path in files:
        normalized = file_path.replace("\\", "/")
        for prefix, scope in (
            ("agent/", "agent"),
            ("center/", "center"),
            ("proto/", "proto"),
            ("deploy/", "deploy"),
            ("experiments/", "experiments"),
            ("docs/", "docs"),
            ("latex/", "latex"),
            ("scripts/", "scripts"),
            (".agent/", "agents"),
            ("figures/", "figures"),
        ):
            if normalized.startswith(prefix):
                scopes.add(scope)
                break
    if len(scopes) == 1:
        return next(iter(scopes))
    if len(scopes) > 1:
        return "p3"
    return None


def summarize_change(status: ChangeStatus) -> str:
    parts: list[str] = []
    if status.added:
        parts.append(f"新增 {len(status.added)} 个文件")
    if status.modified:
        parts.append(f"修改 {len(status.modified)} 个文件")
    if status.deleted:
        parts.append(f"删除 {len(status.deleted)} 个文件")
    if status.untracked:
        parts.append(f"未跟踪 {len(status.untracked)} 个文件")
    return "，".join(parts) or "更新项目文件"


def format_file_list(files: list[str], max_items: int = 6) -> str:
    if len(files) <= max_items:
        return ", ".join(files)
    return f"{', '.join(files[:max_items])} 等 {len(files)} 个"


def generate_fallback_commit_message(status: ChangeStatus) -> str:
    files = status.all_files
    commit_type = infer_commit_type(files)
    scope = infer_scope(files)
    title_body = summarize_change(status)
    title = f"{commit_type}({scope}): {title_body}" if scope else f"{commit_type}: {title_body}"

    details: list[str] = []
    if status.added:
        details.append(f"- 新增: {format_file_list(status.added)}")
    if status.modified:
        details.append(f"- 修改: {format_file_list(status.modified)}")
    if status.deleted:
        details.append(f"- 删除: {format_file_list(status.deleted)}")
    if status.untracked:
        details.append(f"- 未跟踪: {format_file_list(status.untracked)}")
    if not details:
        return title
    return f"{title}\n\n" + "\n".join(details)


def should_use_ai(
    status: ChangeStatus,
    config: dict[str, str],
    ollama: tuple[str, str] | None = None,
    dashscope_ready: bool = True,
) -> tuple[bool, str]:
    if config.get("MYGIT_NO_AI") == "1" or config.get("MYGIT_FAST_RULES") == "1":
        return False, "fast-mode"
    files = status.all_files
    if files and all(is_binary_artifact(f) for f in files):
        return False, "binary-only"
    if any(is_binary_artifact(f) for f in files) and config.get("MYGIT_FORCE_AI") != "1":
        return False, "binary-mixed"
    # 至少有一个可用后端（Ollama / DashScope）
    if ollama is not None:
        return True, "ai-ollama"
    if dashscope_ready:
        return True, "ai-dashscope"
    return False, "no-key"


def get_staged_diff_for_files(text_files: list[str]) -> str:
    """逐文件获取暂存区 diff，跳过二进制与解码失败内容。"""
    if not text_files:
        return ""
    parts: list[str] = []
    for file_path in text_files:
        result = subprocess.run(
            ["git", "diff", "--cached", "--no-ext-diff", "--", file_path],
            capture_output=True,
        )
        if not result.stdout:
            continue
        chunk = result.stdout.decode("utf-8", errors="replace")
        if "Binary files" in chunk or "GIT binary patch" in chunk:
            parts.append(f"# 二进制/不可文本 diff: {file_path}")
            continue
        parts.append(chunk)
    return "\n".join(parts)


def build_staged_diff(text_files: list[str], binary_files: list[str], stat: str, diff: str) -> str:
    parts: list[str] = []
    if binary_files:
        parts.append(
            "# 二进制/大文件（仅列出路径，不含 diff 内容）\n"
            + "\n".join(f"- {f}" for f in binary_files)
        )
    if stat:
        parts.append(stat)
    if diff:
        parts.append(diff)
    return "\n\n".join(parts)


def git_executable_for_push() -> str:
    # WSL 下 GitHub 推送优先 WSL Git + 代理，Windows Git 无代理易 Connection reset
    if is_wsl_linux():
        return "git"
    if win_git_available():
        return WIN_GIT
    return "git"


def build_git_push_env(
    base_env: dict[str, str],
    proxy_url: str | None,
    github_token: str | None,
    *,
    use_proxy: bool = True,
) -> tuple[dict[str, str], list[str]]:
    env = base_env.copy()
    extra: list[str] = []

    if github_token:
        if use_proxy:
            apply_proxy_env(env, proxy_url)
        env["GIT_TERMINAL_PROMPT"] = "0"
        helper = (
            f"!f() {{ echo username=x-access-token; echo password={github_token}; }}; f"
        )
        extra.extend(["-c", f"credential.helper={helper}"])
        return env, extra

    if use_proxy:
        apply_proxy_env(env, proxy_url)
    env["GIT_TERMINAL_PROMPT"] = "0"
    extra.append("-c")
    extra.append("http.version=HTTP/1.1")
    if os.path.isfile(GCM_WRAPPER):
        extra.extend(["-c", f"credential.helper=!{GCM_WRAPPER}"])
    return env, extra


def build_push_command(
    git_bin: str,
    extra_git_args: list[str],
    has_upstream: bool,
    remote: str,
    branch: str,
    proxy_url: str | None,
) -> list[str]:
    cmd = [git_bin, *extra_git_args]
    if proxy_url:
        cmd.extend(
            [
                "-c",
                f"http.proxy={proxy_url}",
                "-c",
                f"https.proxy={proxy_url}",
            ]
        )
    if has_upstream:
        cmd.extend(["push", "--no-verify"])
    else:
        cmd.extend(["push", "--set-upstream", remote, branch, "--no-verify"])
    return cmd


def run_git(args: list[str], env: dict | None = None, check: bool = True) -> subprocess.CompletedProcess:
    result = subprocess.run(["git", *args], capture_output=True, text=True, env=env)
    if check and result.returncode != 0:
        stderr = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(stderr or f"git {' '.join(args)} failed")
    return result


def stage_changes(workspace: str) -> None:
    run_git(["add", "-A"])
    for prefix in AUTO_COMMIT_EXCLUDE_PREFIXES:
        try:
            run_git(["reset", "HEAD", "--", prefix], check=False)
        except RuntimeError:
            pass
    for file_name in AUTO_COMMIT_NEVER_FILES:
        try:
            run_git(["reset", "HEAD", "--", file_name], check=False)
        except RuntimeError:
            pass


def check_version_files(changes_raw: str) -> bool:
    changed_files = [
        parse_porcelain_path(line)[1]
        for line in changes_raw.splitlines()
        if line
    ]
    return any(vf in changed for vf in VERSION_FILES for changed in changed_files)


def has_unpushed_commits() -> bool:
    output = run_command("git cherry -v", check=False) or ""
    return bool(output.strip())


def push_to_remote(proxy_url: str | None, github_token: str | None) -> None:
    print("🚀 正在推送到远程仓库...")
    branch = run_command("git rev-parse --abbrev-ref HEAD") or "main"
    remote = run_command(f"git config branch.{branch}.remote") or "origin"
    remote_url = run_command(f"git remote get-url {remote}") or ""
    has_upstream = bool(run_command(f"git config branch.{branch}.merge"))
    is_github = "github.com" in remote_url.lower()

    print(f"📡 远程仓库: {remote}, 分支: {branch}")

    strategies: list[tuple[str, str, dict[str, str], list[str]]] = []

    if github_token:
        env, extra = build_git_push_env(
            os.environ, proxy_url, github_token, use_proxy=True
        )
        strategies.append(("WSL Git + GITHUB_TOKEN + 代理", "git", env, extra))

    env, extra = build_git_push_env(os.environ, proxy_url, None, use_proxy=True)
    strategies.append(("WSL Git + 代理", "git", env, extra))

    if is_wsl_linux() and win_git_available() and not is_github:
        strategies.append(
            (
                "Windows Git（内网远程）",
                WIN_GIT,
                clean_env_for_windows_git(),
                [],
            )
        )
    elif is_wsl_linux() and win_git_available() and is_github:
        strategies.append(
            (
                "Windows Git（无代理，备选）",
                WIN_GIT,
                clean_env_for_windows_git(),
                [],
            )
        )

    last_error = ""
    for label, git_bin, push_env, extra_git_args in strategies:
        push_cmd = build_push_command(
            git_bin, extra_git_args, has_upstream, remote, branch, proxy_url
        )
        print(f"🔐 尝试: {label}")
        try:
            result = subprocess.run(
                push_cmd, env=push_env, text=True, capture_output=True
            )
        except OSError as exc:
            last_error = str(exc)
            print(f"⚠️  {label} 不可用: {exc}")
            continue

        if result.returncode == 0:
            out = (result.stdout or result.stderr).strip()
            if out:
                print(out)
            print("\n✨ 推送成功！")
            return

        last_error = (result.stderr or result.stdout).strip()
        print(f"⚠️  {label} 失败: {last_error.splitlines()[-1] if last_error else 'unknown'}")

    print(f"\n❌ 推送失败: {last_error}")
    print("本地提交已保留。可尝试：")
    print("  1) 在 .env.local 添加 GITHUB_TOKEN=<GitHub PAT>")
    print("  2) 确认 Clash/V2Ray 代理端口与 .env.mygit 中 MYGIT_HTTP_PROXY 一致")
    print("  3) 手动执行: HTTPS_PROXY=http://127.0.0.1:7890 git push")
    sys.exit(1)


def main() -> None:
    if len(sys.argv) >= 3:
        workspace = os.path.abspath(sys.argv[1])
        script_dir = os.path.abspath(sys.argv[2])
    else:
        workspace = os.getcwd()
        script_dir = os.path.dirname(os.path.abspath(__file__))
        
    os.chdir(workspace)
    project_name = os.path.basename(workspace)
    print(f"🚀 AI Git 提交工具启动 ({project_name})")

    config = load_config(workspace, script_dir)
    api_key = config.get("DASHSCOPE_API_KEY", "").strip()
    base_url = config.get("DASHSCOPE_BASE_URL", "").rstrip("/")
    model = config.get("DASHSCOPE_MODEL", "").strip()
    github_token = config.get("GITHUB_TOKEN") or config.get("GH_TOKEN")

    # 探测本地 Ollama（OpenAI 兼容端点）
    ollama = detect_ollama(config)
    dashscope_ready = bool(api_key and api_key not in PLACEHOLDER_API_KEYS and base_url and model)

    if ollama is None and not dashscope_ready:
        print("❌ 错误: 未找到可用的 AI 后端")
        print("   - 本地 Ollama 未运行或未安装 gemma4:e4b")
        print("   - 云端 DashScope 缺少 DASHSCOPE_API_KEY / DASHSCOPE_BASE_URL / DASHSCOPE_MODEL")
        print("   - 或设 MYGIT_NO_AI=1 走纯规则模式")
        sys.exit(1)

    if ollama is not None:
        ollama_base, ollama_model = ollama
        print(f"🦙 检测到本地 Ollama：{ollama_base} · model={ollama_model}（优先使用）")
        print("⏳ 正在预热本地模型（首次加载约 10-30s）...")
        warmup_ollama(ollama_base, ollama_model)
        print("✅ Ollama 预热完成")
    elif dashscope_ready:
        print(f"☁️  使用云端 DashScope：{base_url} · model={model}")

    proxy_url = resolve_proxy(config)
    if proxy_url:
        print(f"📡 使用代理: {proxy_url}")
    else:
        print("📡 未检测到本地代理端口，将尝试直连")

    if run_command("git rev-parse --git-dir") is None:
        print("❌ 错误: 当前目录不是 Git 仓库")
        sys.exit(1)

    print("📝 正在检查代码变更...")
    status_output = run_command("git status --porcelain") or ""
    if not status_output:
        if has_unpushed_commits():
            print("ℹ️  工作区无变更，但检测到本地有尚未推送的提交，正在同步推送...\n")
            push_to_remote(proxy_url, github_token)
            return
        print("✅ 工作区是干净的，且本地提交已与远程仓库完全同步。")
        return

    status = parse_git_status(status_output)
    if not status.has_changes:
        if has_unpushed_commits():
            print("ℹ️  暂无可提交变更，但检测到本地有尚未推送的提交，正在同步推送...\n")
            push_to_remote(proxy_url, github_token)
            return
        if status.excluded:
            print("✅ 没有可提交的代码变更（已排除构建/索引目录）")
        else:
            print("✅ 工作区是干净的，且本地提交已与远程仓库完全同步。")
        return

    print(f"\n发现 {len(status.all_files)} 个文件变更：")
    for line in status_output.splitlines():
        if not line:
            continue
        line_status, file_path = parse_porcelain_path(line)
        if file_path in status.excluded:
            continue
        print(format_status_line(line_status, file_path))
    if status.excluded:
        print(f"  已跳过: {', '.join(status.excluded)}")

    if check_version_files(status_output):
        print("\n⚠️  警告：检测到版本/部署配置相关文件变更")
        print("   涉及: go.mod、docker-compose 等")
        print("   若本次为版本发布，请先确认依赖与镜像标签是否一致。")
        print("   按回车继续普通提交，Ctrl+C 取消...")
        try:
            input()
        except KeyboardInterrupt:
            print("\n已取消提交")
            sys.exit(0)

    print("\n📦 正在添加变更到暂存区...")
    stage_changes(workspace)

    staged_names = (run_command("git diff --cached --name-only", check=False) or "").splitlines()
    text_files = [f for f in staged_names if f and not is_binary_artifact(f)]
    binary_files = [f for f in staged_names if f and is_binary_artifact(f)]
    stat = run_command("git diff --cached --stat", check=False) or ""

    # 本地 Ollama 端（8B Q4 在 CPU 上推理较慢）应使用更短的 diff 输入，
    # 以免主调用 timeout。截断阈值由 DIFF_PROMPT_MAX_CHARS 控制。
    diff_max_chars = int(config.get("DIFF_PROMPT_MAX_CHARS", "4000"))
    diff_content = get_staged_diff_for_files(text_files)
    if len(diff_content) > diff_max_chars:
        diff_content = diff_content[:diff_max_chars] + "\n... (Diff truncated)"

    use_ai, ai_reason = should_use_ai(status, config, ollama=ollama, dashscope_ready=dashscope_ready)
    commit_msg = ""
    source_label = "规则生成"
    user_prompt = (
        f"变更摘要:\n{status_output}\n\n"
        f"变更详情:\n{build_staged_diff(text_files, binary_files, stat, diff_content)}"
    )
    system_prompt = P3_SYSTEM_PROMPT.replace("p3-microservice", project_name)

    if use_ai:
        # 决定使用哪个后端：Ollama 优先，DashScope 备选
        backends: list[tuple[str, str, str]] = []  # (label, url, model_name)
        if ollama is not None:
            ollama_base, ollama_model = ollama
            backends.append(("Ollama", f"{ollama_base}/chat/completions", ollama_model))
        if dashscope_ready:
            backends.append(("DashScope", f"{base_url}/chat/completions", model))

        commit_msg = ""
        for backend_label, backend_url, backend_model in backends:
            print(f"🤖 正在使用 {backend_label}（{backend_model}）生成提交信息...")
            try:
                # 本地 Ollama 端偏好短回复；云端 DashScope 可保持 500
                default_max_tokens = 200 if backend_label == "Ollama" else 500
                max_tokens = int(
                    config.get(
                        "OLLAMA_MAX_TOKENS" if backend_label == "Ollama" else "DASHSCOPE_MAX_TOKENS",
                        str(default_max_tokens),
                    )
                )
                payload = {
                    "model": backend_model,
                    "messages": [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": user_prompt},
                    ],
                    "max_tokens": max_tokens,
                    "temperature": 0.7,
                }
                session = requests.Session()
                if backend_label == "Ollama":
                    resp = call_ollama_api(session, backend_url, payload)
                else:
                    headers = {
                        "Content-Type": "application/json",
                        "Authorization": f"Bearer {api_key}",
                    }
                    resp = call_dashscope_api(session, backend_url, headers, payload, proxy_url)
                commit_msg = strip_markdown_fence(
                    resp.json()["choices"][0]["message"]["content"]
                )
                source_label = f"AI 生成（{backend_label}）"
                break
            except Exception as exc:
                print(f"⚠️  {backend_label} 生成失败 ({exc})，{'切换到下一后端' if len(backends) > 1 else '降级到托底'}...")
                continue

        if not commit_msg:
            print("⚠️  所有 AI 后端均失败，正在使用托底逻辑...")
            today = datetime.now().strftime("%Y-%m-%d")
            commit_msg = (
                f"chore: 自动同步代码变更 ({today})\n\n"
                f"变更摘要：\n{summarize_change(status)}\n\n"
                "由于 AI 生成失败，此信息由系统自动生成。"
            )
            source_label = "托底生成"
    else:
        reason_map = {
            "fast-mode": "快速模式（MYGIT_NO_AI）",
            "no-key": "未配置有效 AI 后端",
            "binary-only": "变更均为二进制文件",
            "binary-mixed": "含二进制文件（设 MYGIT_FORCE_AI=1 可强制 AI）",
        }
        print(f"📋 使用规则生成提交信息（{reason_map.get(ai_reason, ai_reason)}）...")
        commit_msg = generate_fallback_commit_message(status)
        source_label = "规则生成"

    print(f"\n提交信息 ({source_label})：")
    print("──────────────────────────────────────────────────")
    print(commit_msg)
    print("──────────────────────────────────────────────────\n")

    print("💾 正在创建提交...")
    msg_file = os.path.join(".git", "COMMIT_MSG_TMP")
    with open(msg_file, "w", encoding="utf-8") as f:
        f.write(commit_msg)
    try:
        run_git(["commit", "-F", msg_file, "--no-verify"])
    except RuntimeError as exc:
        if "nothing to commit" in str(exc).lower():
            print("✅ 没有新的变更需要提交")
            sys.exit(0)
        print(f"❌ 提交失败: {exc}")
        sys.exit(1)
    finally:
        if os.path.exists(msg_file):
            os.remove(msg_file)

    push_to_remote(proxy_url, github_token)
    print("✨ 提交并推送成功！")


if __name__ == "__main__":
    main()
