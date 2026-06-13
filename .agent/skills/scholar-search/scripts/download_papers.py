#!/usr/bin/env python3
"""
download_papers.py — 科研论文 PDF 自动下载与归档工具

从搜索结果 JSON 中提取论文元数据，自动下载可获取的 PDF 并归档。

下载源优先级：
  1. arXiv 预印本（通过 arxiv 官方库）
  2. Semantic Scholar 开放获取链接
  3. Unpaywall API（合法开放获取查询）

用法:
  python download_papers.py --input results.json --output-dir ./papers
  python download_papers.py --input results.json --max-downloads 5
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import sys
import time
from pathlib import Path
from typing import Any

import httpx
from dotenv import load_dotenv

# ---------------------------------------------------------------------------
# 加载环境变量
# ---------------------------------------------------------------------------
_PROJECT_ROOT = Path(__file__).resolve().parents[4]
_ENV_FILE = _PROJECT_ROOT / '.env.scholar'
if _ENV_FILE.exists():
    load_dotenv(_ENV_FILE)

_DEFAULT_OUTPUT_DIR = _PROJECT_ROOT / os.environ.get(
    'SCHOLAR_PAPERS_DIR', 'data/papers'
)

# Unpaywall 需要一个邮箱来标识请求者（免费，无需 Key）
_UNPAYWALL_EMAIL = 'scholar-search-tool@example.com'


# ---------------------------------------------------------------------------
# 辅助工具
# ---------------------------------------------------------------------------

def _sanitize_filename(text: str, max_len: int = 80) -> str:
    """将文本转化为安全的文件名片段。"""
    text = re.sub(r'[^\w\s-]', '', text)
    text = re.sub(r'\s+', '_', text.strip())
    return text[:max_len]


def _make_paper_filename(paper: dict[str, Any]) -> str:
    """生成标准化的论文文件名: [年份]_[第一作者姓]_[简化标题].pdf"""
    year = paper.get('year') or 'nodate'
    first_author = 'unknown'
    if paper.get('authors'):
        # 取第一作者的姓
        name_parts = paper['authors'][0].split()
        if name_parts:
            first_author = _sanitize_filename(name_parts[-1], 20)

    title_short = _sanitize_filename(paper.get('title', 'untitled'), 50)
    return f'{year}_{first_author}_{title_short}.pdf'


def _content_hash(paper: dict[str, Any]) -> str:
    """基于 DOI 或标题生成唯一哈希，用于去重。"""
    identifier = paper.get('doi') or paper.get('title', '')
    return hashlib.sha256(identifier.encode()).hexdigest()[:16]


# ---------------------------------------------------------------------------
# 索引管理
# ---------------------------------------------------------------------------

def _load_index(index_path: Path) -> dict[str, Any]:
    """加载已下载论文的索引文件。"""
    if index_path.exists():
        return json.loads(index_path.read_text(encoding='utf-8'))
    return {'papers': {}, 'total_count': 0}


def _save_index(index_path: Path, index: dict[str, Any]) -> None:
    """保存索引文件。"""
    index_path.write_text(
        json.dumps(index, ensure_ascii=False, indent=2),
        encoding='utf-8',
    )


# ---------------------------------------------------------------------------
# 下载器：arXiv
# ---------------------------------------------------------------------------

def _download_from_arxiv(arxiv_id: str, output_path: Path) -> bool:
    """通过 arxiv 库下载 arXiv 预印本。"""
    try:
        import arxiv

        client = arxiv.Client()
        search = arxiv.Search(id_list=[arxiv_id])
        results = list(client.results(search))
        if not results:
            return False

        paper = results[0]
        paper.download_pdf(dirpath=str(output_path.parent), filename=output_path.name)
        return output_path.exists()
    except ImportError:
        print('⚠️  arxiv 库未安装，跳过 arXiv 下载', file=sys.stderr)
        return False
    except Exception as exc:
        print(f'⚠️  arXiv 下载失败 ({arxiv_id}): {exc}', file=sys.stderr)
        return False


# ---------------------------------------------------------------------------
# 下载器：直接 HTTP
# ---------------------------------------------------------------------------

def _download_from_url(url: str, output_path: Path) -> bool:
    """直接通过 HTTP 下载 PDF 文件。"""
    if not url:
        return False

    try:
        with httpx.stream('GET', url, follow_redirects=True, timeout=60) as resp:
            if resp.status_code != 200:
                return False

            # 验证是否是 PDF
            content_type = resp.headers.get('content-type', '')
            if 'pdf' not in content_type and 'octet-stream' not in content_type:
                # 读取前几字节验证 PDF magic bytes
                first_bytes = b''
                for chunk in resp.iter_bytes(chunk_size=8):
                    first_bytes = chunk
                    break
                if not first_bytes.startswith(b'%PDF'):
                    return False
                # 重新下载（因为流已经消费）
                return _download_from_url_full(url, output_path)

            output_path.parent.mkdir(parents=True, exist_ok=True)
            with open(output_path, 'wb') as f:
                for chunk in resp.iter_bytes(chunk_size=8192):
                    f.write(chunk)

        return output_path.exists() and output_path.stat().st_size > 1000
    except Exception as exc:
        print(f'⚠️  HTTP 下载失败: {exc}', file=sys.stderr)
        return False


def _download_from_url_full(url: str, output_path: Path) -> bool:
    """完整下载（非流式，用于重试）。"""
    try:
        resp = httpx.get(url, follow_redirects=True, timeout=60)
        if resp.status_code != 200:
            return False
        if not resp.content[:5].startswith(b'%PDF'):
            return False
        output_path.parent.mkdir(parents=True, exist_ok=True)
        output_path.write_bytes(resp.content)
        return output_path.exists() and output_path.stat().st_size > 1000
    except Exception:
        return False


# ---------------------------------------------------------------------------
# 下载器：Unpaywall（合法开放获取查询）
# ---------------------------------------------------------------------------

def _get_unpaywall_pdf_url(doi: str) -> str:
    """通过 Unpaywall API 查询合法开放获取 PDF 链接。"""
    if not doi:
        return ''
    try:
        url = f'https://api.unpaywall.org/v2/{doi}?email={_UNPAYWALL_EMAIL}'
        resp = httpx.get(url, timeout=15)
        if resp.status_code != 200:
            return ''
        data = resp.json()
        best_oa = data.get('best_oa_location') or {}
        return best_oa.get('url_for_pdf', '') or best_oa.get('url', '')
    except Exception:
        return ''


# ---------------------------------------------------------------------------
# 主下载逻辑
# ---------------------------------------------------------------------------

def download_paper(paper: dict[str, Any], output_dir: Path) -> tuple[bool, str]:
    """
    尝试下载单篇论文，按优先级尝试不同来源。

    下载优先级（基于实战可靠性排序）：
      1. 直接 PDF 链接（含 arXiv PDF URL，最可靠）
      2. arXiv 库下载（作为备选，但 API 易被限流）
      3. Unpaywall 开放获取查询

    Returns:
        (是否成功, 下载文件路径或失败原因)
    """
    filename = _make_paper_filename(paper)
    output_path = output_dir / filename

    # 已存在则跳过
    if output_path.exists():
        return True, f'已存在: {output_path}'

    output_dir.mkdir(parents=True, exist_ok=True)

    # 1. 尝试直接 PDF 链接（含 arXiv PDF URL，最快最可靠）
    pdf_url = paper.get('pdf_url', '')
    if pdf_url:
        print(f'  📥 尝试直接 PDF 下载...')
        if _download_from_url(pdf_url, output_path):
            return True, str(output_path)

    # 2. 尝试 arXiv 库下载（备选，API 可能 429 限流）
    arxiv_id = paper.get('arxiv_id', '')
    if arxiv_id:
        print(f'  📥 尝试 arXiv 库下载 ({arxiv_id})...')
        if _download_from_arxiv(arxiv_id, output_path):
            return True, str(output_path)

    # 3. 尝试 Unpaywall
    doi = paper.get('doi', '')
    if doi:
        print(f'  📥 尝试 Unpaywall 开放获取查询 (DOI: {doi})...')
        oa_url = _get_unpaywall_pdf_url(doi)
        if oa_url:
            if _download_from_url(oa_url, output_path):
                return True, str(output_path)

    return False, '无可用的开放获取 PDF 下载源'


def download_papers(
    papers: list[dict[str, Any]],
    output_dir: Path,
    max_downloads: int = 0,
) -> dict[str, Any]:
    """
    批量下载论文。

    Args:
        papers: 论文元数据列表
        output_dir: 输出目录
        max_downloads: 最大下载数（0 = 不限制）

    Returns:
        下载统计信息
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    index_path = output_dir / 'papers_index.json'
    index = _load_index(index_path)

    stats = {'total': len(papers), 'downloaded': 0, 'skipped': 0, 'failed': 0}
    results: list[dict[str, Any]] = []

    for idx, paper in enumerate(papers, 1):
        if max_downloads > 0 and stats['downloaded'] >= max_downloads:
            print(f'\n📊 已达到最大下载数 ({max_downloads})，停止下载')
            break

        title = paper.get('title', 'Unknown')[:60]
        print(f'\n[{idx}/{len(papers)}] 📄 {title}')

        # 去重检查
        paper_hash = _content_hash(paper)
        if paper_hash in index.get('papers', {}):
            print(f'  ⏭️  已在索引中，跳过')
            stats['skipped'] += 1
            continue

        success, detail = download_paper(paper, output_dir)

        if success:
            print(f'  ✅ {detail}')
            stats['downloaded'] += 1
            index['papers'][paper_hash] = {
                'title': paper.get('title'),
                'authors': paper.get('authors', []),
                'year': paper.get('year'),
                'doi': paper.get('doi', ''),
                'filename': _make_paper_filename(paper),
                'downloaded_at': time.strftime('%Y-%m-%dT%H:%M:%S'),
            }
        else:
            print(f'  ❌ {detail}')
            stats['failed'] += 1

        results.append({
            'title': paper.get('title'),
            'success': success,
            'detail': detail,
        })

        # 礼貌性延迟
        time.sleep(1)

    index['total_count'] = len(index.get('papers', {}))
    _save_index(index_path, index)

    return stats


# ---------------------------------------------------------------------------
# 输出统计
# ---------------------------------------------------------------------------

def print_stats(stats: dict[str, Any], output_dir: Path) -> None:
    """打印下载统计。"""
    print('\n' + '=' * 60)
    print('📊 下载统计')
    print('=' * 60)
    print(f'  总计: {stats["total"]} 篇')
    print(f'  ✅ 成功下载: {stats["downloaded"]} 篇')
    print(f'  ⏭️  跳过(已存在): {stats["skipped"]} 篇')
    print(f'  ❌ 下载失败: {stats["failed"]} 篇')
    print(f'  📂 存储目录: {output_dir}')
    print('=' * 60)


# ---------------------------------------------------------------------------
# CLI 入口
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description='科研论文 PDF 自动下载与归档工具',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
示例:
  # 从搜索结果下载论文
  %(prog)s --input results.json

  # 指定输出目录和最大下载数
  %(prog)s --input results.json --output-dir ./papers --max-downloads 5

  # 使用默认路径（doctor/paper1/data/papers/）
  %(prog)s --input results.json
        ''',
    )
    parser.add_argument('--input', '-i', required=True, help='搜索结果 JSON 文件路径')
    parser.add_argument(
        '--output-dir', '-o',
        default=None,
        help=f'PDF 存储目录（默认: {_DEFAULT_OUTPUT_DIR}）',
    )
    parser.add_argument('--max-downloads', '-m', type=int, default=0,
                        help='最大下载数量（0 = 不限制，默认 0）')

    args = parser.parse_args()

    # 加载搜索结果
    input_path = Path(args.input)
    if not input_path.exists():
        print(f'❌ 输入文件不存在: {input_path}', file=sys.stderr)
        sys.exit(1)

    papers = json.loads(input_path.read_text(encoding='utf-8'))
    if not isinstance(papers, list):
        print('❌ 输入文件格式错误：期望 JSON 数组', file=sys.stderr)
        sys.exit(1)

    output_dir = Path(args.output_dir) if args.output_dir else _DEFAULT_OUTPUT_DIR

    print(f'📦 准备下载 {len(papers)} 篇论文')
    print(f'📂 存储目录: {output_dir}')
    if args.max_downloads > 0:
        print(f'🔢 最大下载数: {args.max_downloads}')
    print()

    stats = download_papers(papers, output_dir, args.max_downloads)
    print_stats(stats, output_dir)


if __name__ == '__main__':
    main()
