<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **Tex2Doc** (7,887 symbols, 13,451 relationships, 300 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## MCP Tools

| Tool | Use for |
|------|---------|
| `query` | Find execution flows by concept (replaces grep for architecture exploration) |
| `context` | Full symbol context: callers, callees, process participation |
| `impact` | Blast radius before editing: direct callers, affected flows, risk level |
| `detect_changes` | Scope check before committing: changed symbols and affected processes |
| `rename` | Safe rename that understands the call graph |
| `cypher` | Direct graph queries |
| `list_repos` | List all indexed repositories |

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, use the `impact` MCP tool with `direction: "upstream"` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `detect_changes` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `query` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `context`.

## Never Do

- NEVER edit a function, class, or method without first running `impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `rename` which understands the call graph.
- NEVER commit changes without running `detect_changes` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/Tex2Doc/context` | Codebase overview, check index freshness |
| `gitnexus://repo/Tex2Doc/clusters` | All functional areas |
| `gitnexus://repo/Tex2Doc/processes` | All execution flows |
| `gitnexus://repo/Tex2Doc/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |

<!-- gitnexus:end -->
