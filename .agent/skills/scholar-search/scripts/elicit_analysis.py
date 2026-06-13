#!/usr/bin/env python3
"""
elicit_analysis.py — Elicit 文献分析工具（可选模块）

通过 Elicit API 获取针对特定科研问题的文献综述与自动提取的核心结论。
需要设置 ELICIT_API_KEY 环境变量。

用法:
  python elicit_analysis.py --query "Does multi-agent RL improve manufacturing quality?"
  python elicit_analysis.py --query "What methods improve edge image quality assessment?" --output analysis.json
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path

import httpx
from dotenv import load_dotenv

# ---------------------------------------------------------------------------
# 加载环境变量
# ---------------------------------------------------------------------------
_PROJECT_ROOT = Path(__file__).resolve().parents[4]
_ENV_FILE = _PROJECT_ROOT / '.env.scholar'
if _ENV_FILE.exists():
    load_dotenv(_ENV_FILE)


# ---------------------------------------------------------------------------
# Elicit API
# ---------------------------------------------------------------------------

_ELICIT_SEARCH_URL = 'https://elicit.com/api/v1/search'


def query_elicit(
    query: str,
    num_results: int = 5,
) -> dict:
    """
    调用 Elicit API 获取针对特定科学问题的文献综述与提取的核心结论。

    Args:
        query: 科研问题，例如 "Does X improve Y?"
        num_results: 返回文献数量

    Returns:
        Elicit 返回的完整 JSON 响应，包含论文、摘要和 Takeaway
    """
    api_key = os.environ.get('ELICIT_API_KEY', '')
    if not api_key:
        return {
            'error': True,
            'message': (
                '未配置 ELICIT_API_KEY。\n'
                '请在 .env.scholar 中设置 ELICIT_API_KEY，'
                '或参阅 resources/env.scholar.template 模板。'
            ),
        }

    headers = {
        'Authorization': f'Bearer {api_key}',
        'Content-Type': 'application/json',
    }
    payload = {
        'query': query,
        'limit': num_results,
    }

    try:
        resp = httpx.post(
            _ELICIT_SEARCH_URL,
            json=payload,
            headers=headers,
            timeout=60,
        )
        resp.raise_for_status()
        return resp.json()
    except httpx.HTTPStatusError as exc:
        return {
            'error': True,
            'message': f'Elicit API HTTP 错误: {exc.response.status_code} — {exc.response.text}',
        }
    except httpx.RequestError as exc:
        return {
            'error': True,
            'message': f'Elicit API 请求失败: {exc}',
        }


# ---------------------------------------------------------------------------
# 格式化输出
# ---------------------------------------------------------------------------

def print_elicit_results(result: dict) -> None:
    """格式化输出 Elicit 分析结果。"""
    if result.get('error'):
        print(f'❌ {result["message"]}', file=sys.stderr)
        return

    try:
        from rich.console import Console
        from rich.panel import Panel
        from rich.markdown import Markdown

        console = Console()

        # 输出综述总结
        if result.get('summary'):
            console.print(Panel(
                Markdown(result['summary']),
                title='📋 Elicit 综述总结',
                border_style='cyan',
            ))

        # 输出每篇论文
        papers = result.get('papers', result.get('results', []))
        for idx, paper in enumerate(papers, 1):
            title = paper.get('title', 'Unknown')
            takeaway = paper.get('takeaway', paper.get('tldr', ''))
            authors = paper.get('authors', '')
            year = paper.get('year', '')

            content = f'**{title}**\n\n'
            if authors:
                content += f'👤 {authors}\n'
            if year:
                content += f'📅 {year}\n'
            if takeaway:
                content += f'\n💡 **核心发现**: {takeaway}\n'

            console.print(Panel(content, title=f'论文 #{idx}', border_style='green'))

    except ImportError:
        # 纯文本降级
        if result.get('summary'):
            print(f'📋 综述总结:\n{result["summary"]}\n')
        papers = result.get('papers', result.get('results', []))
        for idx, paper in enumerate(papers, 1):
            print(f'\n--- 论文 #{idx} ---')
            print(f'标题: {paper.get("title", "Unknown")}')
            if paper.get('takeaway'):
                print(f'核心发现: {paper["takeaway"]}')


# ---------------------------------------------------------------------------
# CLI 入口
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description='Elicit 文献分析工具 — 针对特定科研问题获取文献综述和核心结论',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
示例:
  # 分析科研问题
  %(prog)s --query "Does multi-agent RL improve manufacturing efficiency?"

  # 保存结果到 JSON
  %(prog)s --query "What methods improve edge image quality?" --output analysis.json
        ''',
    )
    parser.add_argument('--query', '-q', required=True, help='科研问题（英文效果更佳）')
    parser.add_argument('--num', '-n', type=int, default=5, help='返回文献数量（默认 5）')
    parser.add_argument('--output', '-o', default=None, help='输出 JSON 文件路径')

    args = parser.parse_args()

    print(f'🔬 正在分析: "{args.query}"')
    print()

    result = query_elicit(args.query, args.num)

    print_elicit_results(result)

    if args.output and not result.get('error'):
        out_path = Path(args.output)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')
        print(f'\n💾 分析结果已保存到: {out_path}')


if __name__ == '__main__':
    main()
