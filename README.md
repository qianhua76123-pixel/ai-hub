<!-- vibecore-meta {"category":"desktop-app","tags":["tauri","rust","react","ai-proxy","cost-tracking","local-first"],"summary":"本地优先的 AI 工具统一管理与代理桌面应用"} -->
# ai-hub

> 一站式本地 AI 代理与用量追踪桌面端

## 这是什么
面向重度 AI 开发者与创作者的 Local-first 桌面客户端。解决多模型/多工具订阅分散、API 账单不透明、流量无法统一归因的痛点。通过 Tauri 将 Rust 后端与 React 前端打包，所有数据与代理逻辑严格驻留本地，提供零配置的 AI 流量拦截、成本核算与智能路由。

## 核心功能
- **零配置扫描** —— 自动发现 Claude Code、Cursor 等 48+ AI 工具的环境变量与配置，免去手动录入
- **透明代理** —— 本地 HTTP/SSE 代理无感拦截所有 API 请求，实时记录流量与延迟
- **精准计费** —— 内置 28+ 模型定价表，结合 Cache Read/Write 状态计算单次请求真实成本
- **智能路由** —— 基于 Prompt 分类与基准测试推荐最优模型，降低调用开销
- **多 Agent 并行** —— 同一 Prompt 并发投递多提供商，横向对比输出质量
- **项目归因** —— 通过 Git 分支自动识别上下文，将 API 消耗精准分摊到具体项目

## 技术架构
- **前端交互层 (React 19 + TS + Tailwind 4)**：负责路由分发、数据可视化与主题管理。入口 `src/main.tsx` 挂载 `react-router-dom`，按业务拆分为 `Dashboard`、`Providers`、`Billing` 等页面。通过 `@tauri-apps/api` 调用 Rust 命令，`recharts` 渲染用量图表。
- **本地代理与业务层 (Rust + Axum + rusqlite)**：核心逻辑驻留 `src-tauri/src/`。`proxy.rs` 实现透明 HTTP 代理与 SSE 流转发；`traffic.rs` 解析 Claude Code JSONL/Cursor DB 等历史日志；`engine.rs` 调度 8 家提供商 API 执行多 Agent 任务；`router.rs` 与 `pricing.rs` 负责 Prompt 分类与成本计算。
- **数据与安全层 (SQLite + macOS Keychain)**：`db.rs` 基于 SQLite 存储结构化用量数据，`conversations.rs` 利用 FTS5 实现全量对话检索。`keystore.rs` 调用系统级 Keychain 加密存储 API Key，`health.rs` 实现熔断器与自动故障转移，保障代理高可用。

## 关键技术决策
- **Tauri v2 替代 Electron** —— 放弃 Chromium 沙箱，换取 <10MB 的二进制体积与原生系统调用能力（Keychain、本地端口绑定），严格契合 Local-first 与零遥测的隐私定位。
- **Rust Axum 内建透明代理** —— 不依赖外部抓包工具，将流量拦截、SSE 流解析、计费逻辑下沉至 Rust 编译层，避免 Node.js 事件循环阻塞，保障高并发下的低延迟与内存安全。
- **Git Branch 自动归因** —— 放弃手动打标签，直接读取工作区 `.git/HEAD`，将 AI 消耗与开发上下文强绑定，解决团队报销与 ROI 核算的断点问题。

## 技术路线 Roadmap

### ✅ 已完成
- Tauri v2 基础骨架与 React 19 路由体系搭建 (`src/main.tsx`, `src/pages/`)
- 本地透明代理核心链路 (`proxy.rs`) 与 SSE 流转发支持
- 28+ 模型定价表与缓存感知计费逻辑 (`pricing.rs`)
- macOS Keychain 安全存储集成 (`keystore.rs`) 与基础 UI 主题切换 (`lib/theme.ts`)

### 🚧 进行中
- 多提供商并发任务引擎 (`engine.rs`) 的容错重试与结果聚合逻辑
- Prompt 分类器 (`router.rs`) 的基准测试数据接入与模型推荐调优
- FTS5 对话全文检索 (`conversations.rs`) 的增量索引构建与前端搜索组件联调

### 🗺 待规划
- 跨平台密钥存储：补全 Windows/Linux 的 Keychain 替代方案（如 Secret Service API / Windows Credential Manager）
- 独立 CLI 模式：提供 `ai-hub proxy start` 无头运行支持，适配 CI/CD 或服务器环境
- 团队同步协议：在保持 Local-first 前提下，设计端到端加密的用量数据导出/同步格式

## 当前状态
版本 `v0.3.0`，核心代理链路与计费模块已跑通，前端路由完整覆盖主要业务场景。已知风险：`router.rs` 的 Prompt 分类依赖本地启发式规则，准确率需随 `scripts/update-benchmarks.py` 持续迭代；当前强依赖 macOS 环境，Windows/Linux 的密钥管理模块尚未落地。

## 目录结构
```
ai-hub/
├─ src/              — React 前端源码（路由、页面、组件、主题与工具函数）
├─ src-tauri/        — Tauri 后端 Rust 工程（代理、数据库、引擎、安全模块）
├─ docs/             — PRD 与产品需求文档
├─ scripts/          — 基准测试数据更新脚本 (`update-benchmarks.py`)
├─ .github/workflows/— CI/CD 流水线配置
├─ package.json      — 前端依赖、Vite 构建与 Tauri CLI 脚本入口
└─ vitest.config.ts  — 前端单元测试与组件测试配置
```

## 快速开始
```bash
# Prerequisites: Node.js 18+, Rust 1.70+, pnpm
pnpm install
pnpm tauri dev
```
- 若需启用完整代理功能，确保系统网络设置将 HTTP/HTTPS 代理指向 `127.0.0.1`（应用启动后会自动配置或提供指引）
- macOS 用户首次运行需在“系统设置 > 隐私与安全性”中授权本地网络访问