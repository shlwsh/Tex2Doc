#!/usr/bin/env python3
"""
scholar_search.py — 科研文献搜索工具

支持三后端搜索引擎：
  1. SerpApi（Google Scholar 封装，需 SERPAPI_API_KEY）
  2. Semantic Scholar（免费，无需 Key，有速率限制）
  3. OpenAlex（免费，无需 Key，无严格速率限制，推荐兜底）

用法:
  python scholar_search.py --query "multi-agent reinforcement learning" --num 5
  python scholar_search.py --query "robotics manipulation" --backend openalex --year-from 2023
  python scholar_search.py --query "edge computing IQA" --output results.json --bibtex refs.bib
"""

from __future__ import annotations

import argparse
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
_PROJECT_ROOT = Path(__file__).resolve().parents[4]  # .agent/skills/scholar-search/scripts -> 项目根
_ENV_FILE = _PROJECT_ROOT / '.env.scholar'
if _ENV_FILE.exists():
    load_dotenv(_ENV_FILE)


# ---------------------------------------------------------------------------
# 辅助工具
# ---------------------------------------------------------------------------

def _sanitize_filename(text: str, max_len: int = 60) -> str:
    """将文本转化为安全的文件名片段。"""
    text = re.sub(r'[^\w\s-]', '', text)
    text = re.sub(r'\s+', '_', text.strip())
    return text[:max_len]


def _year_filter(year: int | None, year_from: int | None) -> bool:
    """判断论文是否满足年份筛选条件。"""
    if year_from is None or year is None:
        return True
    return year >= year_from


# ---------------------------------------------------------------------------
# Semantic Scholar 后端（免费）
# ---------------------------------------------------------------------------

_S2_SEARCH_URL = 'https://api.semanticscholar.org/graph/v1/paper/search'
_S2_FIELDS = 'title,abstract,year,citationCount,authors,externalIds,isOpenAccess,openAccessPdf,url'

_MAX_RETRIES = 5
_RETRY_BACKOFF = 5  # 秒（Semantic Scholar 免费层限制严格，需要较长退避）


def _s2_headers() -> dict[str, str]:
    key = os.environ.get('SEMANTIC_SCHOLAR_API_KEY', '')
    headers: dict[str, str] = {}
    if key:
        headers['x-api-key'] = key
    return headers


def search_semantic_scholar(
    query: str,
    num_results: int = 10,
    year_from: int | None = None,
) -> list[dict[str, Any]]:
    """通过 Semantic Scholar API 搜索文献。"""
    params: dict[str, Any] = {
        'query': query,
        'limit': min(num_results, 100),
        'fields': _S2_FIELDS,
    }
    if year_from:
        params['year'] = f'{year_from}-'

    for attempt in range(1, _MAX_RETRIES + 1):
        try:
            resp = httpx.get(
                _S2_SEARCH_URL,
                params=params,
                headers=_s2_headers(),
                timeout=30,
            )
            if resp.status_code == 429:
                wait = _RETRY_BACKOFF * attempt
                print(f'⏳ Semantic Scholar 速率限制，{wait}s 后重试 ({attempt}/{_MAX_RETRIES})...',
                      file=sys.stderr)
                time.sleep(wait)
                continue
            resp.raise_for_status()
            data = resp.json()
            break
        except httpx.HTTPStatusError as exc:
            if attempt == _MAX_RETRIES:
                print(f'❌ Semantic Scholar API 错误: {exc}', file=sys.stderr)
                return []
            time.sleep(_RETRY_BACKOFF * attempt)
    else:
        return []

    papers: list[dict[str, Any]] = []
    for item in data.get('data', []):
        year = item.get('year')
        if not _year_filter(year, year_from):
            continue

        # 提取 DOI
        ext_ids = item.get('externalIds') or {}
        doi = ext_ids.get('DOI', '')
        arxiv_id = ext_ids.get('ArXiv', '')

        # 提取开放获取 PDF
        oa_pdf = ''
        if item.get('isOpenAccess') and item.get('openAccessPdf'):
            oa_pdf = item['openAccessPdf'].get('url', '')

        authors = [a.get('name', '') for a in (item.get('authors') or [])]

        papers.append({
            'title': item.get('title', ''),
            'authors': authors,
            'year': year,
            'citations': item.get('citationCount', 0),
            'abstract': (item.get('abstract') or '')[:500],
            'doi': doi,
            'arxiv_id': arxiv_id,
            'pdf_url': oa_pdf,
            'url': item.get('url', ''),
            'source': 'semantic-scholar',
        })

    papers.sort(key=lambda x: x.get('citations', 0), reverse=True)
    return papers[:num_results]


# ---------------------------------------------------------------------------
# OpenAlex 后端（完全免费，无严格速率限制）
# ---------------------------------------------------------------------------

_OPENALEX_SEARCH_URL = 'https://api.openalex.org/works'
# OpenAlex 中 arXiv 的 Source ID
_OPENALEX_ARXIV_SOURCE_ID = 's4306400194'


def search_openalex(
    query: str,
    num_results: int = 10,
    year_from: int | None = None,
    arxiv_only: bool = False,
) -> list[dict[str, Any]]:
    """通过 OpenAlex API 搜索文献（完全免费，无需 Key）。

    Args:
        query: 搜索词
        num_results: 返回数量
        year_from: 起始年份
        arxiv_only: 仅返回 arXiv 来源论文（保证可下载 PDF）
    """
    params: dict[str, Any] = {
        'search': query,
        'per_page': min(num_results, 50),
        'sort': 'cited_by_count:desc',
        'select': 'id,doi,title,publication_year,cited_by_count,authorships,open_access,primary_location,abstract_inverted_index',
    }

    # 组合过滤条件（OpenAlex 用逗号连接多个 filter）
    filters: list[str] = []
    if year_from:
        filters.append(f'from_publication_date:{year_from}-01-01')
    if arxiv_only:
        filters.append(f'primary_location.source.id:{_OPENALEX_ARXIV_SOURCE_ID}')
    if filters:
        params['filter'] = ','.join(filters)

    # OpenAlex 礼貌请求：提供邮箱可获得更高速率
    headers = {'User-Agent': 'scholar-search-tool/1.0 (mailto:scholar-search@example.com)'}

    try:
        resp = httpx.get(
            _OPENALEX_SEARCH_URL,
            params=params,
            headers=headers,
            timeout=30,
        )
        resp.raise_for_status()
        data = resp.json()
    except Exception as exc:
        print(f'❌ OpenAlex API 错误: {exc}', file=sys.stderr)
        return []

    papers: list[dict[str, Any]] = []
    for item in data.get('results', []):
        # 提取作者
        authors = []
        for authorship in (item.get('authorships') or []):
            author_obj = authorship.get('author', {})
            name = author_obj.get('display_name', '')
            if name:
                authors.append(name)

        # 提取 DOI
        doi_raw = item.get('doi') or ''
        doi = doi_raw.replace('https://doi.org/', '') if doi_raw else ''

        # 提取开放获取 PDF
        oa_pdf = ''
        oa_info = item.get('open_access') or {}
        if oa_info.get('is_oa'):
            oa_pdf = oa_info.get('oa_url', '')

        # 重建摘要（OpenAlex 使用倒排索引存储摘要）
        abstract = ''
        abs_inv = item.get('abstract_inverted_index')
        if abs_inv and isinstance(abs_inv, dict):
            try:
                word_positions: list[tuple[int, str]] = []
                for word, positions in abs_inv.items():
                    for pos in positions:
                        word_positions.append((pos, word))
                word_positions.sort()
                abstract = ' '.join(w for _, w in word_positions)[:500]
            except Exception:
                pass

        # 提取 arXiv ID（从 primary_location）
        arxiv_id = ''
        primary_loc = item.get('primary_location') or {}
        source = primary_loc.get('source') or {}
        if source.get('display_name', '').lower() == 'arxiv':
            landing_url = primary_loc.get('landing_page_url', '')
            if 'arxiv.org/abs/' in landing_url:
                arxiv_id = landing_url.split('arxiv.org/abs/')[-1]
            # arXiv 来源时，确保 pdf_url 指向 arXiv PDF（比 OA URL 更可靠）
            if arxiv_id:
                oa_pdf = f'https://arxiv.org/pdf/{arxiv_id}'

        papers.append({
            'title': item.get('title', ''),
            'authors': authors,
            'year': item.get('publication_year'),
            'citations': item.get('cited_by_count', 0),
            'abstract': abstract,
            'doi': doi,
            'arxiv_id': arxiv_id,
            'pdf_url': oa_pdf,
            'url': item.get('id', ''),
            'source': 'openalex',
        })

    papers.sort(key=lambda x: x.get('citations', 0), reverse=True)
    return papers[:num_results]


# ---------------------------------------------------------------------------
# SerpApi 后端（Google Scholar）
# ---------------------------------------------------------------------------

def _has_serpapi() -> bool:
    """检查 SerpApi 是否可用。"""
    if not os.environ.get('SERPAPI_API_KEY'):
        return False
    try:
        from serpapi import GoogleSearch  # noqa: F401
        return True
    except ImportError:
        return False


def search_serpapi(
    query: str,
    num_results: int = 10,
    year_from: int | None = None,
    lang: str = 'en',
) -> list[dict[str, Any]]:
    """通过 SerpApi 调用 Google Scholar 搜索。"""
    from serpapi import GoogleSearch

    params: dict[str, Any] = {
        'engine': 'google_scholar',
        'q': query,
        'hl': lang,
        'num': min(num_results, 20),
        'api_key': os.environ['SERPAPI_API_KEY'],
    }
    if year_from:
        params['as_ylo'] = year_from

    search = GoogleSearch(params)
    results = search.get_dict()

    papers: list[dict[str, Any]] = []
    for item in results.get('organic_results', []):
        cited_by = item.get('inline_links', {}).get('cited_by', {})
        citations = cited_by.get('total', 0) if isinstance(cited_by, dict) else 0

        # 尝试提取作者和年份
        pub_info = item.get('publication_info', {})
        authors_str = pub_info.get('authors', [])
        authors = (
            [a.get('name', '') for a in authors_str]
            if isinstance(authors_str, list) else []
        )
        year = pub_info.get('year')

        papers.append({
            'title': item.get('title', ''),
            'authors': authors,
            'year': int(year) if year else None,
            'citations': citations,
            'abstract': (item.get('snippet') or '')[:500],
            'doi': '',
            'arxiv_id': '',
            'pdf_url': item.get('resources', [{}])[0].get('link', '') if item.get('resources') else '',
            'url': item.get('link', ''),
            'source': 'serpapi',
        })

    papers.sort(key=lambda x: x.get('citations', 0), reverse=True)
    return papers[:num_results]


# ---------------------------------------------------------------------------
# 关键词相关性打分
# ---------------------------------------------------------------------------

def score_relevance(
    papers: list[dict[str, Any]],
    keywords: list[str],
) -> list[dict[str, Any]]:
    """
    用自定义关键词对文献进行相关性打分并重排序。

    每个关键词匹配得 1 分（标题匹配双倍），引用量对数加分（最多 6 分）。
    打分结果存入 paper['_relevance_score'] 字段。

    Args:
        papers: 文献列表
        keywords: 相关性关键词列表

    Returns:
        按相关性降序排列的文献列表
    """
    import math

    for p in papers:
        title = (p.get('title') or '').lower()
        abstract = (p.get('abstract') or '').lower()
        text = f'{title} {abstract}'
        score = 0.0
        matched: list[str] = []

        for kw in keywords:
            kw_lower = kw.lower()
            if kw_lower in title:
                score += 2  # 标题匹配双倍分
                matched.append(kw)
            elif kw_lower in text:
                score += 1
                matched.append(kw)

        # 引用量加分（对数缩放，最多 6 分）
        cit = p.get('citations', 0)
        if cit > 0:
            score += min(math.log10(cit) * 1.5, 6)

        p['_relevance_score'] = round(score, 1)
        p['_matched_keywords'] = matched

    papers.sort(key=lambda x: x.get('_relevance_score', 0), reverse=True)
    return papers


# ---------------------------------------------------------------------------
# 统一搜索入口
# ---------------------------------------------------------------------------

def search(
    query: str,
    num_results: int = 10,
    year_from: int | None = None,
    backend: str = 'auto',
    lang: str = 'en',
    arxiv_only: bool = False,
    relevance_keywords: list[str] | None = None,
) -> list[dict[str, Any]]:
    """
    统一文献搜索入口。

    Args:
        query: 搜索词
        num_results: 返回数量
        year_from: 起始年份筛选
        backend: 'serpapi' | 'semantic-scholar' | 'openalex' | 'auto'
        lang: 搜索语言（仅 SerpApi 使用）
        arxiv_only: 仅返回 arXiv 来源论文（保证可下载，仅 OpenAlex 后端支持）
        relevance_keywords: 自定义相关性关键词列表，用于对结果重排序

    Returns:
        按引用量或相关性降序排列的文献列表
    """
    papers: list[dict[str, Any]] = []

    if backend == 'serpapi':
        if not _has_serpapi():
            print('⚠️  SerpApi 不可用（缺少 API Key 或未安装包），降级到 OpenAlex',
                  file=sys.stderr)
            papers = search_openalex(query, num_results, year_from, arxiv_only)
        else:
            papers = search_serpapi(query, num_results, year_from, lang)
    elif backend == 'semantic-scholar':
        papers = search_semantic_scholar(query, num_results, year_from)
    elif backend == 'openalex':
        papers = search_openalex(query, num_results, year_from, arxiv_only)
    else:
        # auto: SerpApi -> Semantic Scholar -> OpenAlex
        if _has_serpapi():
            try:
                papers = search_serpapi(query, num_results, year_from, lang)
            except Exception as exc:
                print(f'⚠️  SerpApi 调用失败 ({exc})，尝试下一个后端', file=sys.stderr)

        if not papers:
            try:
                papers = search_semantic_scholar(query, num_results, year_from)
            except Exception as exc:
                print(f'⚠️  Semantic Scholar 调用失败 ({exc})，降级到 OpenAlex', file=sys.stderr)

        if not papers:
            print('ℹ️  使用 OpenAlex 后端搜索', file=sys.stderr)
            papers = search_openalex(query, num_results, year_from, arxiv_only)

    # 相关性打分（可选）
    if relevance_keywords and papers:
        papers = score_relevance(papers, relevance_keywords)
        # 打分后截取 num_results（因为可能请求了更多用于排序）
        papers = papers[:num_results]

    return papers


# ---------------------------------------------------------------------------
# BibTeX 生成
# ---------------------------------------------------------------------------

def _to_bibtex_key(paper: dict[str, Any]) -> str:
    """生成 BibTeX cite key。"""
    first_author = ''
    if paper.get('authors'):
        first_author = paper['authors'][0].split()[-1].lower()
    year = paper.get('year') or 'nodate'
    title_word = _sanitize_filename(paper.get('title', 'untitled').split()[0], 10).lower()
    return f'{first_author}{year}{title_word}'


def papers_to_bibtex(papers: list[dict[str, Any]]) -> str:
    """将文献列表转为 BibTeX 格式。"""
    entries = []
    seen_keys: set[str] = set()
    for p in papers:
        key = _to_bibtex_key(p)
        # 去重
        orig_key = key
        counter = 1
        while key in seen_keys:
            key = f'{orig_key}_{counter}'
            counter += 1
        seen_keys.add(key)

        authors_str = ' and '.join(p.get('authors', ['Unknown']))
        entry = (
            f'@article{{{key},\n'
            f'  title  = {{{p.get("title", "")}}},\n'
            f'  author = {{{authors_str}}},\n'
            f'  year   = {{{p.get("year", "")}}},\n'
        )
        if p.get('doi'):
            entry += f'  doi    = {{{p["doi"]}}},\n'
        if p.get('url'):
            entry += f'  url    = {{{p["url"]}}},\n'
        if p.get('abstract'):
            abstract_clean = p['abstract'].replace('{', '').replace('}', '')
            entry += f'  abstract = {{{abstract_clean}}},\n'
        entry += '}'
        entries.append(entry)

    return '\n\n'.join(entries) + '\n'


# ---------------------------------------------------------------------------
# Rich 表格输出
# ---------------------------------------------------------------------------

def print_results_table(papers: list[dict[str, Any]]) -> None:
    """在终端用 Rich 彩色表格输出结果。"""
    try:
        from rich.console import Console
        from rich.table import Table
    except ImportError:
        # Rich 不可用时降级到纯文本
        _print_results_plain(papers)
        return

    console = Console()
    table = Table(
        title='📚 文献搜索结果',
        show_lines=True,
        title_style='bold cyan',
    )
    table.add_column('#', style='dim', width=3)
    table.add_column('标题', style='bold', max_width=50)
    table.add_column('作者', max_width=30)
    table.add_column('年份', justify='center', width=6)
    table.add_column('引用', justify='right', width=6, style='green')
    table.add_column('来源', width=8)
    table.add_column('PDF', width=4, justify='center')

    for idx, p in enumerate(papers, 1):
        authors_display = ', '.join(p.get('authors', [])[:3])
        if len(p.get('authors', [])) > 3:
            authors_display += ' et al.'
        pdf_icon = '✅' if p.get('pdf_url') else '❌'
        table.add_row(
            str(idx),
            p.get('title', ''),
            authors_display,
            str(p.get('year', '')),
            str(p.get('citations', 0)),
            p.get('source', ''),
            pdf_icon,
        )

    console.print(table)
    console.print(f'\n🔍 共找到 [bold]{len(papers)}[/bold] 篇文献\n')


def _print_results_plain(papers: list[dict[str, Any]]) -> None:
    """纯文本输出（Rich 不可用时的降级）。"""
    print('=' * 80)
    print('📚 文献搜索结果')
    print('=' * 80)
    for idx, p in enumerate(papers, 1):
        authors_display = ', '.join(p.get('authors', [])[:3])
        if len(p.get('authors', [])) > 3:
            authors_display += ' et al.'
        pdf_mark = '[PDF]' if p.get('pdf_url') else '[NO PDF]'
        print(f'\n{idx}. {p.get("title", "")}')
        print(f'   作者: {authors_display}')
        print(f'   年份: {p.get("year", "N/A")}  引用: {p.get("citations", 0)}  {pdf_mark}')
        if p.get('abstract'):
            print(f'   摘要: {p["abstract"][:150]}...')
    print(f'\n🔍 共找到 {len(papers)} 篇文献')


# ---------------------------------------------------------------------------
# CLI 入口
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        description='科研文献搜索工具 — 支持 SerpApi (Google Scholar) 和 Semantic Scholar 双后端',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog='''
示例:
  # 基础搜索（自动选择后端）
  %(prog)s --query "multi-agent reinforcement learning"

  # 使用 Semantic Scholar 免费后端，限定 2023 年后，返回 10 条
  %(prog)s --query "robot manipulation" --backend semantic-scholar --year-from 2023 --num 10

  # 输出 JSON + BibTeX 文件
  %(prog)s --query "edge computing" --output results.json --bibtex refs.bib
        ''',
    )
    parser.add_argument('--query', '-q', required=True, help='搜索关键词')
    parser.add_argument('--num', '-n', type=int, default=10, help='返回文献数量（默认 10）')
    parser.add_argument('--year-from', type=int, default=None, help='限制起始年份（如 2023）')
    parser.add_argument(
        '--backend', '-b',
        choices=['auto', 'serpapi', 'semantic-scholar', 'openalex'],
        default='auto',
        help='搜索后端（默认 auto：SerpApi -> Semantic Scholar -> OpenAlex 逐级降级）',
    )
    parser.add_argument('--lang', default='en', help='搜索语言，仅 SerpApi 使用（默认 en）')
    parser.add_argument('--output', '-o', default=None, help='输出 JSON 文件路径')
    parser.add_argument('--bibtex', default=None, help='输出 BibTeX 文件路径')
    parser.add_argument('--json-only', action='store_true', help='仅输出 JSON 到 stdout，不显示表格')
    parser.add_argument(
        '--arxiv-only', action='store_true',
        help='仅返回 arXiv 来源论文（保证可下载 PDF，仅 OpenAlex 后端支持）',
    )
    parser.add_argument(
        '--relevance-keywords', '-k', nargs='+', default=None,
        help='自定义相关性关键词列表，用于对结果重排序（例如: IQA tongue edge robot）',
    )

    args = parser.parse_args()

    print(f'🔍 正在搜索: "{args.query}" (后端: {args.backend}, 数量: {args.num})')
    if args.year_from:
        print(f'📅 限定年份: {args.year_from} 年至今')
    print()

    papers = search(
        query=args.query,
        num_results=args.num,
        year_from=args.year_from,
        backend=args.backend,
        lang=args.lang,
        arxiv_only=args.arxiv_only,
        relevance_keywords=args.relevance_keywords,
    )

    if not papers:
        print('❌ 未找到任何文献，请尝试调整搜索词或后端。')
        sys.exit(1)

    # 输出表格 / JSON
    if args.json_only:
        print(json.dumps(papers, ensure_ascii=False, indent=2))
    else:
        print_results_table(papers)

    # 保存 JSON
    if args.output:
        out_path = Path(args.output)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(json.dumps(papers, ensure_ascii=False, indent=2), encoding='utf-8')
        print(f'💾 结果已保存到: {out_path}')

    # 保存 BibTeX
    if args.bibtex:
        bib_path = Path(args.bibtex)
        bib_path.parent.mkdir(parents=True, exist_ok=True)
        bib_content = papers_to_bibtex(papers)
        bib_path.write_text(bib_content, encoding='utf-8')
        print(f'📖 BibTeX 已保存到: {bib_path}')


if __name__ == '__main__':
    main()
