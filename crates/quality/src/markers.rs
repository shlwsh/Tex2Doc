//! 22 个 marker 列表（V1 沿用 + V2 扩到三列）。

use crate::layer::MarkerHit;
use crate::normalize::normalize;

/// 与 `docs/to-docx/08-verification.md §8.4` 一致的 22 marker。
pub const MARKERS: &[&str] = &[
    "网关流量驱动的微服务定向日志采集框架",  // 标题
    "摘  要", "关键词", "Abstract", "Key words",                    // 摘要标签
    "1 引言", "2 相关工作", "3 系统总体设计", "4 关键算法",         // 章节
    "5 系统实现", "6 实验与分析", "7 结束语",
    "表 1", "表 5", "图 1", "图 8", "算法 1",                       // 表/图/算法
    "References", "附中文参考文献", "作者简介",                     // 参考/简介
    "shihonglei0042@link.tyut.edu.cn", "zh_juanjuan@126.com",        // 邮箱
];

/// 在三处同时检测 marker 命中。
pub fn coverage(docx: &str, oracle: &str, rust: &str) -> Vec<MarkerHit> {
    let d = normalize(docx);
    let o = normalize(oracle);
    let r = normalize(rust);
    MARKERS
        .iter()
        .map(|m| {
            let n = normalize(m);
            MarkerHit {
                marker: m.to_string(),
                in_docx: d.contains(&n),
                in_oracle_pdf: o.contains(&n),
                in_rust_pdf: r.contains(&n),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markers_hit_all_three_sides() {
        let docx = "摘  要 网关流量驱动的微服务定向日志采集框架 关键词 1 引言 Abstract Key words 2 相关工作 3 系统总体设计 4 关键算法 5 系统实现 6 实验与分析 7 结束语 表 1 表 5 图 1 图 8 算法 1 References 附中文参考文献 作者简介 shihonglei0042@link.tyut.edu.cn zh_juanjuan@126.com";
        let oracle = docx;
        let rust = docx;
        let hits = coverage(docx, oracle, rust);
        assert_eq!(hits.len(), MARKERS.len());
        assert!(hits.iter().all(|h| h.in_docx && h.in_oracle_pdf && h.in_rust_pdf));
    }
}
