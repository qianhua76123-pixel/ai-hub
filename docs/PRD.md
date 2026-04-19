# AI Hub — 产品需求文档 (PRD)

> 版本: 0.3.0 | 日期: 2026-04-09 | 作者: AI Hub Team

---

## 1. 产品定位

**一句话描述：** 本地桌面应用，统一管理所有 AI 工具的订阅、用量、费用和智能路由。

**核心价值主张：**
- **零配置自动检测** — 打开即用，自动发现本机所有 AI 工具
- **本地优先隐私** — 所有数据存本地，API Key 加密保存（macOS Keychain），不上传不经过第三方
- **精准费用追踪** — 区分 cache read/write/input/output，精确到每次请求
- **实时评测数据** — 自动从 Arena AI、OpenRouter 拉取最新模型评分和价格
- **智能任务路由** — 根据任务类型自动选择最优模型
- **费用预警** — 预算上限 + 实时预警，防止意外超支

**目标用户：** 同时使用 3+ AI 工具的开发者和重度 AI 用户

**品牌定位：** 本地、免费、全工具覆盖

**竞品差异化：**
| 维度 | cc-switch | OpenRouter | Helicone | LiteLLM | AICosts.ai | **AI Hub** |
|------|-----------|------------|----------|---------|------------|----------|
| 本地运行 | ✅ | ❌ | ❌ | 可选 | ❌ | ✅ |
| 自动检测工具 | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| 费用追踪 | 基础 | ✅ | ✅ | ✅ | ✅ | ✅ cache 级精确 |
| 实时评测排行 | ❌ | 基础 | ❌ | ❌ | ❌ | ✅ 多维度 |
| 智能路由 | ❌ | ✅ | ❌ | ✅ | ❌ | ✅ 本地 |
| 费用预警 | ❌ | ❌ | ✅ | ✅ | ✅ | ✅ |
| 隐私保护 | ✅ | ❌ | ❌ | 可选 | ❌ | ✅ |
| 免费 | ✅ | ❌ (5.5%) | 免费层 | 开源 | $19.99/月 | ✅ |
| 多工具支持 | 5个 | N/A | N/A | N/A | 50+ | ✅ 扩展中 |
| Dark Mode | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |

---

## 2. 已完成功能清单

### v0.1.0 — 基础能力 ✅
| 模块 | 功能 | 状态 |
|------|------|------|
| 工具检测 | 环境变量/配置文件/IDE 插件/.env 自动扫描（48+ providers） | ✅ |
| 流量代理 | 本地 HTTP 代理，SSE streaming 支持，11+ provider 路由 | ✅ |
| 日志解析 | Claude Code JSONL + Codex SQLite + Cursor DB | ✅ |
| 费用计算 | 28+ 模型价格，区分 cache read/write | ✅ |
| 用量统计 | 30 天趋势 + 费用分布 + 24h 活跃度 + 模型明细 | ✅ |
| 订阅管理 | 订阅 vs API 对比 + 模型评比排行榜 | ✅ |
| 搜索 | Cmd+K 全局搜索 | ✅ |
| 自动接入 | 启动时自动配置 Claude Code 走代理 | ✅ |

### v0.2.0 — 智能分析 ✅
| 模块 | 功能 | 状态 |
|------|------|------|
| 智能路由 | Prompt 分类（代码/推理/写作/对话）+ Top 3 推荐 + 路由决策日志 | ✅ |
| 项目归因 | 代理请求自动关联 git 分支 + 项目手动标签 + CSV/JSON 导出 | ✅ |
| 速率预测 | 滑动窗口请求率计算 + 限流预警 + 自动 failover | ✅ |
| ROI 计算器 | 30天数据分析 + 月度费用预测 + 保留/取消建议 | ✅ |
| 对话搜索 | 跨工具全文搜索（FTS5）+ 按来源过滤 | ✅ |
| 质量验证 | 多 Agent 交叉验证（Claude 写 + GPT 审） | ✅ |
| Provider 预设 | 25+ 一键导入 + API Key 加密存储 | ✅ |
| 健康监控 | 断路器 + 后台 ping + Dashboard 状态卡片 | ✅ |
| 实时评测 | Arena Text/Code ELO + OpenRouter 价格自动拉取 | ✅ |
| 实时汇率 | ExchangeRate-API + Frankfurter 双源冗余 | ✅ |
| Dark Mode | 跟随系统/手动切换 + 全 UI 适配 | ✅ |
| 系统托盘 | 后台运行 + 右键菜单 + 双击恢复 | ✅ |
| 首次引导 | 3 步向导 + 自动检测 | ✅ |
| 安全 | Keychain 加密 + CSP + CORS 限制 + 退出恢复配置 | ✅ |
| 测试 | 75 个单元测试（47 Rust + 28 Frontend）+ CI | ✅ |

---

## 3. v0.3.0 需求清单 — 基于市场调研

### P0 — 必须做（用户核心痛点，竞品已有）

#### 3.1 多工具支持扩展
**痛点：** AI Hub 目标用户是"日常使用 3+ AI 工具"的开发者，但代理切换目前仅支持 Claude Code，覆盖面不足。cc-switch 已支持 5 个工具。
**方案：**
- Codex CLI: 读写 `~/.codex/config.json`，设置 `OPENAI_BASE_URL` 环境变量
- Gemini CLI: 设置 `GOOGLE_API_BASE` 或 `GEMINI_API_BASE`
- Cursor: 修改 `~/.cursor/User/settings.json` 的 `openai.baseUrl`
- Aider: 设置 `OPENAI_API_BASE` 环境变量
- Continue: 修改 `~/.continue/config.json` 的 provider baseUrl
- 通用: 提供 shell 环境变量注入（已有 `install_shell_proxy`）

**验收标准：**
- 设置页"工具接入"列出 5+ 个可切换工具
- 每个工具一键接入/断开
- 退出 AI Hub 时自动恢复所有工具的原始配置

---

#### 3.2 实时费用预警与预算控制
**痛点：** 开发者月度 AI 花费 $200-500，但无法实时感知。单次 Claude Code session 可能 $8-20，ChatGPT Plus 重度用户 $200+/月。"用到月中才发现已经超支"是普遍痛点。
**方案：**
- 用户设置月度总预算（如 ¥1000）和单 Provider 预算
- 代理层实时累计当月花费
- 达到 70%/90%/100% 时弹出系统通知（macOS Notification Center）
- 100% 时可选择自动暂停代理（停止转发请求）
- Dashboard 顶部显示预算进度条

**验收标准：**
- 设置页可配置月度预算
- 达到阈值时弹出桌面通知
- Dashboard 显示预算消耗进度

---

#### 3.3 代理自动 Fallback
**痛点：** API 挂了或返回 429 限速，用户工作被中断。OpenRouter/LiteLLM/Portkey 都有自动 fallback。
**方案：**
- 代理层检测到错误/超时/429 时，自动重试到备选 Provider
- 用户可配置 Fallback 链（如 Claude → GPT → DeepSeek）
- Fallback 事件记录到日志，UI 可查看

**验收标准：**
- Provider A 失败时自动切换到 Provider B 重发请求
- 用户无感知，响应正常返回
- Dashboard 健康面板显示 fallback 事件

---

### P1 — 应该做（差异化竞争力）

#### 3.4 本地语义缓存
**痛点：** 相似的 prompt 反复发送，浪费 token。Helicone 号称缓存可降低 95% 成本。
**方案：**
- 代理层对请求 body 做 hash
- 完全相同的请求直接返回缓存（精确匹配）
- 可选：基于 embedding 的近似匹配（需要本地 embedding 模型）
- 缓存 TTL 可配置（默认 1 小时）
- 缓存命中时标记为 `cached`，不计入费用

**验收标准：**
- 完全相同的请求在 TTL 内直接返回
- Dashboard 显示缓存命中率和节省金额
- 设置页可开关缓存、配置 TTL

---

#### 3.5 统一订阅管理仪表板
**痛点：** "订阅蔓延"(subscription sprawl) — 开发者同时订阅 ChatGPT Plus、Claude Pro、Cursor Pro、Copilot，月费 $200+，不清楚哪些值回票价。
**方案：**
- 专属"订阅"页面，统一展示所有 AI 订阅
- 用户手动添加订阅（名称、月费、计费日）
- 自动计算每个订阅的实际使用量和 ROI
- 到期前 3 天提醒
- 一键生成"取消建议"报告

**验收标准：**
- 可添加/编辑/删除订阅
- 每个订阅显示 ROI、使用频率、到期日
- 到期前自动通知

---

#### 3.6 MCP 服务器管理
**痛点：** MCP 已成为 2026 年 AI 工具的标准协议（Linux Foundation 下的开放标准），开发者需要管理多个 MCP 服务器。cc-switch 和 Portkey 已有此功能。
**方案：**
- 扫描本地已安装的 MCP 服务器（`~/.claude/settings.json` 中的 mcpServers）
- 提供统一的启动/停止/状态查看界面
- MCP 流量统计（调用次数、延迟）
- MCP 服务器市场（推荐热门 MCP 服务器）

**验收标准：**
- 自动发现已配置的 MCP 服务器
- 可查看每个 MCP 服务器的状态和调用统计

---

### P2 — 可以做（增长级）

#### 3.7 跨平台费用聚合
- 允许用户手动录入非 API 的 AI 费用（ChatGPT Web、Perplexity、Midjourney 等）
- 统一日历视图展示所有 AI 相关支出
- 月度/季度费用报告

#### 3.8 虚拟 Key 管理
- 为不同项目分配虚拟 API Key
- 每个虚拟 Key 可设置独立配额
- 适合小团队（2-5 人）场景

#### 3.9 Agent 工作流编排
- 定义多步骤 Agent 工作流
- 步骤间自动传递上下文
- 工作流级别的成本追踪

---

## 4. 技术架构

### 当前架构（v0.2.0）
```
                    ┌─────────────────────────────────┐
                    │        AI Hub Desktop            │
                    ├─────────────────────────────────┤
                    │  React 19 + TypeScript + TW 4   │
                    │  7 页面 + Onboarding + Dark Mode │
                    ├─────────────────────────────────┤
                    │  Tauri v2 Rust Backend           │
                    │  ├─ proxy.rs (SSE streaming)     │
                    │  ├─ benchmarks.rs (Arena/OR API) │
                    │  ├─ router.rs (智能路由)          │
                    │  ├─ health.rs (断路器+预测)       │
                    │  ├─ keystore.rs (Keychain)       │
                    │  ├─ pricing.rs (缓存+失效)        │
                    │  ├─ traffic.rs (增量扫描)         │
                    │  ├─ conversations.rs (FTS5)      │
                    │  ├─ presets.rs (25+ Provider)    │
                    │  ├─ switcher.rs (动态端口)        │
                    │  └─ scanner.rs (48+ 检测)        │
                    ├─────────────────────────────────┤
                    │  SQLite + FTS5 + Schema Versioning│
                    │  macOS Keychain · System Tray     │
                    └─────────────────────────────────┘
```

### v0.3.0 新增
```
                    ┌─────────────────────────────────┐
    新增 →          │  预算引擎                         │
                    │  ├─ 月度预算阈值                  │
                    │  ├─ 实时费用累计                  │
                    │  └─ macOS 通知推送               │
                    ├─────────────────────────────────┤
    新增 →          │  多工具 Switcher                  │
                    │  ├─ Codex CLI 适配器             │
                    │  ├─ Gemini CLI 适配器            │
                    │  ├─ Cursor 适配器                │
                    │  ├─ Aider 适配器                 │
                    │  └─ Continue 适配器              │
                    ├─────────────────────────────────┤
    新增 →          │  Proxy Fallback 引擎             │
                    │  ├─ 错误检测 (5xx/429/timeout)   │
                    │  ├─ Fallback 链配置              │
                    │  └─ 自动重试 + 日志              ���
                    └─────────────────────────────────┘
```

### 数据库新增
```sql
-- 预算配置
CREATE TABLE IF NOT EXISTS budgets (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  monthly_limit_usd REAL NOT NULL DEFAULT 0.0,
  provider_id TEXT DEFAULT '',  -- 空=全局预算
  notify_at_70 BOOLEAN DEFAULT 1,
  notify_at_90 BOOLEAN DEFAULT 1,
  auto_pause_at_100 BOOLEAN DEFAULT 0,
  created_at INTEGER NOT NULL
);

-- MCP 服务器
CREATE TABLE IF NOT EXISTS mcp_servers (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL DEFAULT '',
  command TEXT NOT NULL DEFAULT '',
  args TEXT NOT NULL DEFAULT '[]',
  source TEXT NOT NULL DEFAULT '',  -- claude/cursor/manual
  status TEXT NOT NULL DEFAULT 'stopped',
  total_calls INTEGER DEFAULT 0,
  last_call_at INTEGER
);
```

---

## 5. 里程碑计划

| 版本 | 核心功能 | 状态 |
|------|---------|------|
| v0.1.0 | 代理 + 扫描 + 费用追踪 + 任务引擎 | ✅ 已完成 |
| v0.2.0 | 智能路由 + 项目归因 + ROI + 评测 + 安全 + Dark Mode | ✅ 已完成 |
| **v0.3.0** | **多工具支持 + 费用预警 + 自动 Fallback** | 🚧 开发中 |
| v0.4.0 | 本地缓存 + 订阅仪表板 + MCP 管理 | 📋 计划中 |
| v1.0.0 | 代码签名 + 自动更新 + 跨平台 + 稳定发布 | 📋 计划中 |

---

## 6. 成功指标

| 指标 | 目标 | 当前 |
|------|------|------|
| 首次启动到看到数据 | < 5 秒 | ✅ |
| 费用计算误差 | < 2% | ✅ (OpenRouter 实时价格) |
| 代理附加延迟 | < 50ms | ✅ (SSE streaming) |
| 评测数据更新频率 | 每日 | ✅ (启动时自动) |
| 支持工具数 | 5+ | 🚧 (当前 1) |
| 单元测试覆盖 | 核心路径 100% | ✅ 75 tests |

---

## 7. 风险与约束

| 风险 | 缓解措施 |
|------|---------|
| 各 AI 工具配置格式变化 | 模块化适配器 + 快速跟进更新 |
| 用户不信任本地代理 | 开源代码 + 透明日志 + 不修改请求内容 |
| 模型价格频繁变动 | OpenRouter API 实时价格 + Arena 每日更新 |
| 竞品 cc-switch 多工具领先 | v0.3.0 优先补齐多工具支持 |
| macOS Keychain 权限弹窗 | 首次引导说明 + 优雅降级到文件存储 |
| Arena API 不稳定 | 双数据源冗余 + 本地缓存 fallback |

---

## 8. 不做的事（经过市场调研排除）

| 功能 | 排除原因 |
|------|---------|
| 本地模型推理 | 有 Ollama/LM Studio，不重复造轮子 |
| AI 代码质量评分 | 用户要的是更好的 AI，不是评分工具 |
| 社交/社区功能 | 开发者工具不需要社交 |
| 全功能 LLM Gateway | 和 LiteLLM/Portkey 正面竞争是死路 |
| 企业多租户 | 当前定位个人开发者，团队需求后续考虑 |
