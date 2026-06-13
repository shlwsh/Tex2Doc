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


def load_config(workspace: str) -> dict[str, str]:
    config = load_env_file(os.path.join(workspace, ".env.mygit"))
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


def should_use_ai(status: ChangeStatus, config: dict[str, str]) -> tuple[bool, str]:
    if config.get("MYGIT_NO_AI") == "1" or config.get("MYGIT_FAST_RULES") == "1":
        return False, "fast-mode"
    api_key = config.get("DASHSCOPE_API_KEY", "").strip()
    if api_key in PLACEHOLDER_API_KEYS:
        return False, "no-key"
    files = status.all_files
    if files and all(is_binary_artifact(f) for f in files):
        return False, "binary-only"
    if any(is_binary_artifact(f) for f in files) and config.get("MYGIT_FORCE_AI") != "1":
        return False, "binary-mixed"
    return True, "ai"


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
    print("🚀 AI Git 提交工具启动 (p3-microservice)")

    workspace = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    os.chdir(workspace)

    if not os.path.exists(".env.mygit"):
        print("❌ 错误: 找不到配置文件 .env.mygit")
        print("请执行: cp .agent/skills/mygit/resources/env.mygit.template .env.mygit")
        sys.exit(1)

    config = load_config(workspace)
    api_key = config.get("DASHSCOPE_API_KEY", "").strip()
    base_url = config.get("DASHSCOPE_BASE_URL", "").rstrip("/")
    model = config.get("DASHSCOPE_MODEL", "").strip()
    github_token = config.get("GITHUB_TOKEN") or config.get("GH_TOKEN")

    if not all([api_key, base_url, model]):
        print("❌ 错误: 配置缺少 DASHSCOPE_API_KEY / DASHSCOPE_BASE_URL / DASHSCOPE_MODEL")
        sys.exit(1)

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
    diff_content = get_staged_diff_for_files(text_files)
    if len(diff_content) > 15000:
        diff_content = diff_content[:15000] + "\n... (Diff truncated)"

    use_ai, ai_reason = should_use_ai(status, config)
    commit_msg = ""
    source_label = "规则生成"

    if use_ai:
        print("🤖 正在使用 AI 生成提交信息...")
        try:
            headers = {
                "Content-Type": "application/json",
                "Authorization": f"Bearer {api_key}",
            }
            payload = {
                "model": model,
                "messages": [
                    {"role": "system", "content": P3_SYSTEM_PROMPT},
                    {
                        "role": "user",
                        "content": (
                            f"变更摘要:\n{status_output}\n\n"
                            f"变更详情:\n{build_staged_diff(text_files, binary_files, stat, diff_content)}"
                        ),
                    },
                ],
                "max_tokens": 500,
                "temperature": 0.7,
            }
            session = requests.Session()
            resp = call_dashscope_api(
                session,
                f"{base_url}/chat/completions",
                headers,
                payload,
                proxy_url,
            )
            commit_msg = strip_markdown_fence(
                resp.json()["choices"][0]["message"]["content"]
            )
            source_label = "AI 生成"
        except Exception as exc:
            print(f"⚠️  AI 生成提交信息失败 ({exc})，正在使用托底逻辑...")
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
            "no-key": "未配置有效 API Key",
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
