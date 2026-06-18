#!/usr/bin/env python3
"""
更新迭代记录表 (docs/verify/迭代记录.md)

用法:
    python3 scripts/update_iteration_record.py <VERSION> <JSON_PATH> <RECORD_PATH>

示例:
    python3 scripts/update_iteration_record.py \
        v1321 \
        docs/verify/v1321-20260618-120000/docx-compare.json \
        docs/verify/迭代记录.md
"""
import json
import sys
import os
from datetime import datetime

def update_record(version: str, json_path: str, record_path: str):
    with open(json_path) as f:
        r = json.load(f)

    s = r.get("summary", {})

    para_delta   = s.get("paragraph_delta", -999)
    real_diff    = s.get("format_changed_real_paragraphs", -1)
    split_only   = s.get("format_changed_split_only_paragraphs", -1)
    table_delta  = s.get("table_delta", 0)
    date_str     = datetime.now().strftime("%Y-%m-%d %H:%M")

    # 达标判定
    ok_real  = "PASS" if real_diff <= 5 else "FAIL"
    ok_para  = "PASS" if abs(para_delta) <= 5 else "FAIL"
    ok_split = "PASS" if split_only <= 10 else "FAIL"
    overall  = "PASS" if (ok_real == "PASS" and ok_para == "PASS") else "FAIL"

    new_row = (
        f"| {version} | {date_str} | "
        f"{para_delta:+d} | {real_diff} | {split_only} | {table_delta:+d} | "
        f"{ok_real} | {ok_para} | {ok_split} | {overall} |\n"
    )

    if not os.path.exists(record_path):
        # 首次: 创建表头
        header = (
            "| 版本 | 日期 | 段落差 | 真实格式差 | run分割差 | 表格数差 | "
            "格式达标 | 结构达标 | run达标 | 总体 |\n"
            "|------|------|--------|-----------|----------|---------|"
            "--------|---------|-------|--------|\n"
        )
        with open(record_path, "w") as f:
            f.write("# v13.2 质量迭代记录\n\n")
            f.write(header)
        print(f"已创建记录文件: {record_path}")

    with open(record_path, "r") as f:
        content = f.read()

    # 查找最后一行非表头内容
    lines = content.rstrip().split("\n")
    # 找到最后一个 | 开头的非表头行
    last_data_idx = -1
    for i in range(len(lines) - 1, -1, -1):
        if lines[i].strip().startswith("|"):
            last_data_idx = i
            break

    if last_data_idx >= 0:
        lines.insert(last_data_idx + 1, new_row.rstrip())
    else:
        lines.append(new_row.rstrip())

    with open(record_path, "w") as f:
        f.write("\n".join(lines) + "\n")

    print(f"已追加记录: {version}")
    print(f"  段落差={para_delta:+d}, 真实格式差={real_diff}, run分割差={split_only}")
    print(f"  达标: 格式={ok_real}, 结构={ok_para}, run={ok_split}, 总体={overall}")


if __name__ == "__main__":
    if len(sys.argv) < 4:
        print("用法: python3 update_iteration_record.py <VERSION> <JSON_PATH> <RECORD_PATH>")
        sys.exit(1)
    update_record(sys.argv[1], sys.argv[2], sys.argv[3])
