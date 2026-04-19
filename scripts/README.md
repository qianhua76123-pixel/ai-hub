# AI Hub Scripts

## update-benchmarks.py

从公开排行榜抓取最新的 AI 模型评测数据，更新到 `pricing.json`。

### 数据来源

| 来源 | URL | 更新字段 |
|------|-----|---------|
| Arena ELO | https://arena.ai/leaderboard | `arena_score` |
| Aider Polyglot | https://aider.chat/docs/leaderboards/ | `aider_polyglot` |
| SWE-bench | https://www.swebench.com | `swe_bench` |

### 用法

```bash
# 更新所有评测数据
python3 scripts/update-benchmarks.py

# 只看变更，不写入
python3 scripts/update-benchmarks.py --dry-run

# 只更新某个来源
python3 scripts/update-benchmarks.py --source arena
python3 scripts/update-benchmarks.py --source aider
python3 scripts/update-benchmarks.py --source swe
```

### AI 工具集成

此脚本设计为可被 AI 工具（如 Claude Code）调用：

```bash
# 在 Claude Code 中运行
! python3 ~/ai-hub/scripts/update-benchmarks.py --dry-run
```

### 零依赖

仅使用 Python 标准库（urllib, json, re），无需 pip install。
