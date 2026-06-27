# Doc-engine 标准规则缓存策略
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



> 版本：v1.0  
> 日期：2026-06-17  
> 结论：首版不引入 SQLite。标准规则、Profile、映射规则继续以 YAML/JSON 文件作为事实源，`standards.lock.json` 作为可复现锁文件；SQLite cache 仅作为后续性能优化选项。

## 决策

当前转换引擎的标准语法、映射规则和 Profile 具备以下特征：

1. 规则规模小，读入成本低。
2. 需要人工审阅和版本管理。
3. 需要随 Git diff 追踪变更。
4. 首版优先保证可解释性和可复现性。

因此首版采用：

- `standards/tex/*.yaml`：TeX/LaTeX 语法规则。
- `standards/ooxml/*.yaml`：OOXML/OPC 映射目标规则。
- `standards/mappings/*.yaml`：Standard AST 到 DOCX render tree 的映射规则。
- `profiles/jos-2025/*.yaml`：期刊 Profile 和覆盖项。
- `standards.lock.json`：规则文件 SHA-256 锁定。

## SQLite Cache 的保留边界

后续只有在出现以下情况时再引入 SQLite：

- 规则数量扩大到数千条以上，启动加载成为明显瓶颈。
- 需要按 rule id、source span、mapping id 做复杂查询。
- 需要保存多版本规则索引和增量迁移历史。
- 需要把质量报告中的失败项反查到规则库，并支持交互式检索。

即使引入 SQLite，也只作为派生缓存，不作为事实源。生成方式应为：

```text
standards/*.yaml + profiles/*.yaml + standards.lock.json
        -> build cache
        -> .cache/doc-engine/rules.sqlite
```

缓存必须满足：

- 可删除后重建。
- schema_version 写入数据库。
- standards.lock hash 不匹配时拒绝复用。
- CI 默认不依赖缓存。

## 验收

首版验收以文件源为准：

- `standards.lock.json` 可重新生成并稳定。
- AST dump 和 render dump 可输出规则 id。
- quality traceability 报告可列出 AST rule ids 和 render mapping ids。
