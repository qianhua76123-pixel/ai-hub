import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  CreditCard,
  TrendingDown,
  TrendingUp,
  ArrowRightLeft,
  Layers,
  Trophy,
  RefreshCw,
  Loader2,
  Sparkles,
  Info,
  ExternalLink,
  Calculator,
  CheckCircle2,
  XCircle,
  MinusCircle,
} from "lucide-react";
import { cn } from "../lib/utils";

interface ModelPrice {
  provider: string;
  provider_name: string;
  model_id: string;
  model_name: string;
  input_per_m: number;
  output_per_m: number;
  cache_write_per_m: number;
  cache_read_per_m: number;
  context_window: number;
  category: string;
  note: string;
  arena_score: number;
  swe_bench: number;
  aider_polyglot: number;
  humaneval: number;
}

interface SubscriptionPlan {
  provider: string;
  provider_name: string;
  plan_name: string;
  price_monthly_usd: number;
  price_monthly_cny: number;
  includes: string;
  api_equivalent_note: string;
}

interface Comparison {
  provider_name: string;
  plan_name: string;
  subscription_cny: number;
  api_cost_cny: number;
  savings_usd: number;
  savings_cny: number;
  recommendation: string;
}

interface CostData {
  monthly_api_cost_usd: number;
  monthly_api_cost_cny: number;
  comparisons: Comparison[];
}

interface PricingInfo {
  last_updated: string;
  model_count: number;
  source: string;
}

interface RoiResult {
  provider: string;
  provider_name: string;
  plan_name: string;
  subscription_usd: number;
  subscription_cny: number;
  api_cost_usd: number;
  api_cost_cny: number;
  savings_usd: number;
  savings_cny: number;
  roi_percent: number;
  request_count: number;
  recommendation: string;
  cost_per_request: number;
  predicted_next_month_usd: number;
  predicted_next_month_cny: number;
  trend_percent: number;
}

interface RoiData {
  results: RoiResult[];
}

const catLabel: Record<string, string> = { flagship: "旗舰", fast: "快速", mini: "轻量", reasoning: "推理" };
const catColor: Record<string, string> = {
  flagship: "bg-primary/10 text-primary border-primary/20",
  fast: "bg-success/10 text-success border-success/20",
  mini: "bg-warning/10 text-warning border-warning/20",
  reasoning: "bg-primary/10 text-primary-dark border-primary/20",
};
const pColor: Record<string, string> = {
  anthropic: "#d97706", openai: "#10a37f", google: "#4285f4", deepseek: "#4d6bfe",
  kimi: "#6c5ce7", qwen: "#ff6a00", zhipu: "#0052d9", groq: "#f55036",
  mistral: "#ff7000", cursor: "#00d4aa", copilot: "#6e40c9",
};

// 评比数据来源
const benchmarkSources = [
  { name: "LMSYS Arena (综合)", field: "Arena ELO", url: "https://lmarena.ai/?leaderboard" },
  { name: "LMSYS Arena (代码)", field: "Code Arena", url: "https://lmarena.ai/?leaderboard&category=coding" },
  { name: "SWE-bench Verified", field: "SWE-bench", url: "https://www.swebench.com" },
  { name: "Aider Polyglot", field: "Aider", url: "https://aider.chat/docs/leaderboards/" },
  { name: "OpenRouter 价格", field: "实时价格", url: "https://openrouter.ai/models" },
];

export default function Billing() {
  const [models, setModels] = useState<ModelPrice[]>([]);
  const [, setPlans] = useState<SubscriptionPlan[]>([]);
  const [costData, setCostData] = useState<CostData | null>(null);
  const [pricingInfo, setPricingInfo] = useState<PricingInfo | null>(null);
  const [roiData, setRoiData] = useState<RoiData | null>(null);
  const [accountModes, setAccountModes] = useState<{ provider_id: string; mode: string; subscription_monthly_usd: number }[]>([]);
  const [tab, setTab] = useState<"compare" | "models" | "arena" | "plans" | "roi">("compare");
  const [arenaSubTab, setArenaSubTab] = useState<"overall" | "code" | "value" | "cheap">("overall");
  const [syncing, setSyncing] = useState(false);
  const [syncMsg, setSyncMsg] = useState("");
  const [benchUpdating, setBenchUpdating] = useState(false);
  const [benchMsg, setBenchMsg] = useState("");

  useEffect(() => {
    invoke<ModelPrice[]>("get_model_prices").then(setModels);
    invoke<SubscriptionPlan[]>("get_subscription_plans").then(setPlans);
    invoke<CostData>("get_cost_comparison").then(setCostData);
    invoke<PricingInfo>("get_pricing_info").then(setPricingInfo);
    invoke<RoiData>("get_subscription_roi").then(setRoiData);
    invoke<{ provider_id: string; mode: string; subscription_monthly_usd: number }[]>("get_account_modes").then(setAccountModes).catch(() => {});
  }, []);

  async function handleSync() {
    setSyncing(true);
    setSyncMsg("");
    try {
      const msg = await invoke<string>("fetch_latest_pricing");
      setSyncMsg(msg);
      invoke<ModelPrice[]>("get_model_prices").then(setModels);
      invoke<PricingInfo>("get_pricing_info").then(setPricingInfo);
    } catch (e) {
      setSyncMsg("同步失败: " + String(e));
    }
    setSyncing(false);
    setTimeout(() => setSyncMsg(""), 5000);
  }

  // Extract code_elo from note field (set by benchmarks.rs)
  function getCodeElo(m: ModelPrice): number {
    const match = m.note.match(/code_elo:(\d+)/);
    return match ? parseInt(match[1]) : 0;
  }

  // Multi-dimensional rankings
  const arenaOverall = [...models].filter(m => m.arena_score > 0).sort((a, b) => b.arena_score - a.arena_score);
  const arenaCode = [...models].filter(m => getCodeElo(m) > 0).sort((a, b) => getCodeElo(b) - getCodeElo(a));
  // arenaValue and arenaCheap computed inline in the table renderer below

  return (
    <div className="space-y-8">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">订阅管理</h1>
        <div className="flex items-center gap-3">
          {pricingInfo && (
            <span className="text-xs text-text-faint">
              更新于 {pricingInfo.last_updated} · {pricingInfo.model_count} 个模型
            </span>
          )}
          <button
            onClick={handleSync}
            disabled={syncing}
            className="flex items-center gap-2 px-4 py-2 bg-surface-lighter rounded-xl text-sm text-text-muted hover:text-text hover:bg-surface-elevated transition-all"
          >
            {syncing ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
            {syncing ? "同步中..." : "更新价格"}
          </button>
        </div>
      </div>

      {syncMsg && (
        <div className="bg-primary/5 text-primary px-4 py-3 rounded-2xl text-sm border border-primary/20">{syncMsg}</div>
      )}

      {/* Tab 切换 */}
      <div className="flex gap-1 bg-surface-light rounded-xl p-1 border border-border w-fit">
        {[
          { key: "compare" as const, label: "订阅 vs API", icon: ArrowRightLeft },
          { key: "models" as const, label: "模型价格", icon: Layers },
          { key: "arena" as const, label: "模型评比", icon: Trophy },
          { key: "plans" as const, label: "订阅计划", icon: CreditCard },
          { key: "roi" as const, label: "ROI 分析", icon: Calculator },
        ].map((t) => (
          <button
            key={t.key}
            onClick={() => setTab(t.key)}
            className={cn(
              "flex items-center gap-2 px-4 py-2 rounded-lg text-sm transition-all",
              tab === t.key
                ? "bg-surface-lighter text-text font-medium shadow-sm"
                : "text-text-muted hover:text-text"
            )}
          >
            <t.icon size={14} />
            {t.label}
          </button>
        ))}
      </div>

      {/* ===== 订阅 vs API ===== */}
      {tab === "compare" && costData && (
        <div className="space-y-6">
          <div className="bg-surface-light rounded-2xl p-6 border border-border">
            <h2 className="text-sm text-text-muted mb-2">近 30 天 API 总花费</h2>
            <div className="text-4xl font-bold tracking-tight">
              ¥{costData.monthly_api_cost_cny.toFixed(2)}
              <span className="text-base text-text-faint ml-3">${costData.monthly_api_cost_usd.toFixed(2)}</span>
            </div>
          </div>
          {costData.comparisons.length > 0 ? (
            <div className="space-y-4">
              {costData.comparisons.map((c, i) => {
                const saving = c.savings_usd > 0;
                return (
                  <div key={i} className="bg-surface-light rounded-2xl p-6 border border-border hover:shadow-lg hover:-translate-y-0.5 transition-all">
                    <div className="flex items-center justify-between mb-4">
                      <div className="font-medium text-lg">{c.provider_name} · {c.plan_name}</div>
                      <span className={cn("flex items-center gap-1.5 text-sm px-3 py-1.5 rounded-full border",
                        saving ? "bg-success/10 text-success border-success/20" : "bg-primary/10 text-primary border-primary/20")}>
                        {saving ? <TrendingDown size={14} /> : <TrendingUp size={14} />}
                        {c.recommendation}
                      </span>
                    </div>
                    <div className="grid grid-cols-3 gap-4">
                      <div className="bg-surface rounded-xl p-4">
                        <div className="text-text-faint text-xs mb-1">API 花费 (30天)</div>
                        <div className="font-bold text-xl">¥{c.api_cost_cny.toFixed(2)}</div>
                      </div>
                      <div className="bg-surface rounded-xl p-4">
                        <div className="text-text-faint text-xs mb-1">订阅费用 (月)</div>
                        <div className="font-bold text-xl">¥{c.subscription_cny.toFixed(0)}</div>
                      </div>
                      <div className={cn("rounded-xl p-4", saving ? "bg-success/5" : "bg-primary/5")}>
                        <div className="text-text-faint text-xs mb-1">{saving ? "订阅可节省" : "API 更划算"}</div>
                        <div className={cn("font-bold text-xl", saving ? "text-success" : "text-primary")}>
                          ¥{Math.abs(c.savings_cny).toFixed(2)}
                        </div>
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          ) : (
            <div className="bg-surface-light rounded-2xl p-12 border border-border text-center">
              <Sparkles size={32} className="mx-auto mb-4 text-text-faint" />
              <p className="text-text-muted">暂无足够数据进行对比</p>
              <p className="text-sm text-text-faint mt-1">使用 AI 工具一段时间后自动生成</p>
            </div>
          )}
        </div>
      )}

      {/* ===== 模型价格 ===== */}
      {tab === "models" && (
        <div className="space-y-6">
          <div className="bg-accent-teal/5 rounded-2xl p-5 border border-accent-teal/20 flex items-start gap-3">
            <Info size={18} className="text-accent-teal mt-0.5 shrink-0" />
            <div className="text-sm">
              <div className="font-medium text-accent-teal mb-1">缓存价格说明</div>
              <div className="text-text-muted leading-relaxed">
                <strong>缓存命中 (Cache Read)</strong> 比正常输入便宜很多。例如 Opus：正常输入 $15/M → 缓存命中 $1.50/M（<strong>省 90%</strong>）。
                Claude Code 几乎全部使用缓存命中，实际费用远低于按正常输入估算。
              </div>
            </div>
          </div>

          <div className="bg-surface-light rounded-2xl border border-border overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border text-text-faint text-xs">
                  <th className="text-left p-4">模型</th>
                  <th className="text-left p-4">类型</th>
                  <th className="text-right p-4">输入 $/M</th>
                  <th className="text-right p-4">输出 $/M</th>
                  <th className="text-right p-4"><span className="text-warning">缓存写入</span></th>
                  <th className="text-right p-4"><span className="text-success">缓存命中</span></th>
                  <th className="text-right p-4">上下文</th>
                </tr>
              </thead>
              <tbody>
                {models.map((m) => (
                  <tr key={m.model_id} className="border-b border-border/30 hover:bg-surface-lighter/50 transition-colors">
                    <td className="p-4">
                      <div className="flex items-center gap-3">
                        <div className="w-2.5 h-2.5 rounded-full shrink-0" style={{ backgroundColor: pColor[m.provider] || "#666" }} />
                        <div>
                          <div className="font-medium">{m.model_name}</div>
                          <div className="text-xs text-text-faint">{m.provider_name}</div>
                        </div>
                      </div>
                    </td>
                    <td className="p-4">
                      <span className={cn("text-xs px-2 py-1 rounded-lg border", catColor[m.category])}>
                        {catLabel[m.category] || m.category}
                      </span>
                    </td>
                    <td className="p-4 text-right font-mono">{m.input_per_m > 0 ? `$${m.input_per_m.toFixed(2)}` : "免费"}</td>
                    <td className="p-4 text-right font-mono">{m.output_per_m > 0 ? `$${m.output_per_m.toFixed(2)}` : "免费"}</td>
                    <td className="p-4 text-right font-mono text-warning">
                      {m.cache_write_per_m > 0 ? `$${m.cache_write_per_m.toFixed(2)}` : "-"}
                    </td>
                    <td className="p-4 text-right font-mono text-success">
                      {m.cache_read_per_m > 0 ? (
                        <span>
                          ${m.cache_read_per_m < 0.1 ? m.cache_read_per_m.toFixed(3) : m.cache_read_per_m.toFixed(2)}
                          {m.input_per_m > 0 && (
                            <span className="text-xs ml-1 opacity-50">
                              (-{Math.round((1 - m.cache_read_per_m / m.input_per_m) * 100)}%)
                            </span>
                          )}
                        </span>
                      ) : "-"}
                    </td>
                    <td className="p-4 text-right text-text-faint text-xs">
                      {m.context_window >= 1000000 ? (m.context_window / 1000000) + "M" : (m.context_window / 1000) + "K"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p className="text-xs text-text-faint">
            价格单位: 美元 / 百万 Token · 数据文件: ~/Library/Application Support/ai-hub/pricing.json
          </p>
        </div>
      )}

      {/* ===== 模型评比 ===== */}
      {tab === "arena" && (
        <div className="space-y-5">
          {/* 子榜单切换 + 更新按钮 */}
          <div className="flex items-center justify-between">
            <div className="flex gap-1 bg-surface-light rounded-xl p-1 border border-border">
              {([
                { key: "overall", label: "综合排行", icon: Trophy },
                { key: "code", label: "代码能力", icon: Layers },
                { key: "value", label: "性价比之王", icon: TrendingDown },
                { key: "cheap", label: "价格最低", icon: CreditCard },
              ] as const).map((st) => (
                <button key={st.key} onClick={() => setArenaSubTab(st.key)}
                  className={cn("flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs transition-all",
                    arenaSubTab === st.key ? "bg-surface-lighter text-text font-medium shadow-sm" : "text-text-muted hover:text-text")}>
                  <st.icon size={12} /> {st.label}
                </button>
              ))}
            </div>
            <div className="flex items-center gap-2">
              <button onClick={async () => {
                setBenchUpdating(true); setBenchMsg("");
                try { const r = await invoke<string>("run_benchmark_update", { dryRun: false }); setBenchMsg(r);
                  invoke<ModelPrice[]>("get_model_prices").then(setModels);
                } catch(e) { setBenchMsg(String(e)); }
                setBenchUpdating(false);
                setTimeout(() => setBenchMsg(""), 8000);
              }}
                disabled={benchUpdating}
                className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs text-primary bg-primary/8 hover:bg-primary/12 font-medium transition-colors">
                {benchUpdating ? <Loader2 size={12} className="animate-spin" /> : <RefreshCw size={12} />}
                更新评测数据
              </button>
            </div>
          </div>

          {benchMsg && <pre className="bg-surface-lighter rounded-xl p-3 text-[11px] text-text-muted max-h-40 overflow-auto border border-border-light">{benchMsg}</pre>}

          {/* 数据来源 */}
          <div className="flex gap-2 flex-wrap">
            {benchmarkSources.map((src) => (
              <a key={src.name} href={src.url} target="_blank" rel="noopener noreferrer"
                className="text-[11px] px-2.5 py-1 rounded-full bg-surface-lighter border border-border/50 text-text-muted hover:text-primary hover:border-primary/30 transition-all inline-flex items-center gap-1">
                {src.name} <ExternalLink size={9} />
              </a>
            ))}
            <span className="text-[11px] text-text-faint px-2 py-1">{models.length} 个模型</span>
          </div>

          {/* 排行榜表格 */}
          <div className="bg-surface-light rounded-2xl border border-border overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border text-text-faint text-xs">
                  <th className="text-left p-3 w-10">#</th>
                  <th className="text-left p-3">模型</th>
                  {arenaSubTab === "overall" && <><th className="text-center p-3">Arena ELO</th><th className="text-center p-3">SWE-bench</th><th className="text-center p-3">Aider</th><th className="text-center p-3">HumanEval</th></>}
                  {arenaSubTab === "code" && <><th className="text-center p-3">Code Arena</th><th className="text-center p-3">SWE-bench</th><th className="text-center p-3">Aider</th><th className="text-center p-3">HumanEval</th></>}
                  {arenaSubTab === "value" && <><th className="text-center p-3">Arena ELO</th><th className="text-right p-3">均价 $/M</th><th className="text-center p-3">ELO/$ 比</th><th className="text-center p-3">性价比</th></>}
                  {arenaSubTab === "cheap" && <><th className="text-right p-3">输入 $/M</th><th className="text-right p-3">输出 $/M</th><th className="text-center p-3">Arena ELO</th><th className="text-center p-3">上下文</th></>}
                </tr>
              </thead>
              <tbody>
                {(() => {
                  let ranked: (ModelPrice & { _score: number })[] = [];

                  if (arenaSubTab === "overall") {
                    ranked = arenaOverall.map(m => ({ ...m, _score: m.arena_score }));
                  } else if (arenaSubTab === "code") {
                    // Prefer Arena Code ELO when available, fallback to composite score
                    ranked = arenaCode.length > 0
                      ? arenaCode.map(m => ({ ...m, _score: getCodeElo(m) }))
                      : [...models]
                        .filter(m => m.swe_bench > 0 || m.aider_polyglot > 0)
                        .map(m => ({ ...m, _score: Math.round(m.swe_bench * 0.4 + m.aider_polyglot * 0.4 + m.humaneval * 0.2) }))
                        .sort((a, b) => b._score - a._score);
                  } else if (arenaSubTab === "value") {
                    ranked = [...models]
                      .filter(m => m.arena_score > 0 && m.input_per_m > 0)
                      .map(m => {
                        const avg = (m.input_per_m + m.output_per_m) / 2;
                        return { ...m, _score: Math.round(m.arena_score / avg) };
                      })
                      .sort((a, b) => b._score - a._score);
                  } else {
                    ranked = [...models]
                      .filter(m => m.arena_score > 0)
                      .map(m => ({ ...m, _score: -((m.input_per_m + m.output_per_m) / 2) }))
                      .sort((a, b) => {
                        const aAvg = (a.input_per_m + a.output_per_m) / 2;
                        const bAvg = (b.input_per_m + b.output_per_m) / 2;
                        return aAvg - bAvg;
                      });
                  }

                  return ranked.slice(0, 30).map((m, i) => {
                    const avgPrice = (m.input_per_m + m.output_per_m) / 2;
                    const costPerf = avgPrice > 0 ? Math.round(m.arena_score / avgPrice) : Infinity;

                    return (
                      <tr key={m.model_id} className={cn("border-b border-border/30 hover:bg-surface-lighter/50 transition-colors", i === 0 && "bg-warning/3")}>
                        <td className="p-3 font-bold">
                          {i < 3 ? ["🥇", "🥈", "🥉"][i] : <span className="text-text-faint text-xs">{i + 1}</span>}
                        </td>
                        <td className="p-3">
                          <div className="flex items-center gap-2">
                            <div className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: pColor[m.provider] || "#666" }} />
                            <div>
                              <div className="text-[13px] font-medium">{m.model_name}</div>
                              <div className="text-[10px] text-text-faint">{m.provider_name}</div>
                            </div>
                            <span className={cn("text-[10px] px-1.5 py-0.5 rounded border", catColor[m.category])}>{catLabel[m.category]}</span>
                          </div>
                        </td>

                        {arenaSubTab === "overall" && <>
                          <td className="p-3 text-center font-bold text-base">{m.arena_score}</td>
                          <td className="p-3 text-center">{m.swe_bench > 0 ? (
                            <div className="flex items-center justify-center gap-1.5"><div className="w-16 h-1.5 bg-surface rounded-full overflow-hidden"><div className="h-full bg-primary rounded-full" style={{width:`${m.swe_bench}%`}} /></div><span className="text-[10px] w-8">{m.swe_bench}%</span></div>
                          ) : <span className="text-text-faint text-xs">-</span>}</td>
                          <td className="p-3 text-center">{m.aider_polyglot > 0 ? (
                            <div className="flex items-center justify-center gap-1.5"><div className="w-16 h-1.5 bg-surface rounded-full overflow-hidden"><div className="h-full bg-success rounded-full" style={{width:`${m.aider_polyglot}%`}} /></div><span className="text-[10px] w-8">{m.aider_polyglot}%</span></div>
                          ) : <span className="text-text-faint text-xs">-</span>}</td>
                          <td className="p-3 text-center text-xs">{m.humaneval > 0 ? `${m.humaneval}%` : "-"}</td>
                        </>}

                        {arenaSubTab === "code" && <>
                          <td className="p-3 text-center font-bold text-base">{getCodeElo(m) > 0 ? getCodeElo(m) : <span className="text-text-faint text-xs">-</span>}</td>
                          <td className="p-3 text-center">{m.swe_bench > 0 ? <span className="font-medium">{m.swe_bench}%</span> : "-"}</td>
                          <td className="p-3 text-center">{m.aider_polyglot > 0 ? <span className="font-medium">{m.aider_polyglot}%</span> : "-"}</td>
                          <td className="p-3 text-center text-xs">{m.humaneval > 0 ? `${m.humaneval}%` : "-"}</td>
                        </>}

                        {arenaSubTab === "value" && <>
                          <td className="p-3 text-center font-medium">{m.arena_score}</td>
                          <td className="p-3 text-right font-mono text-xs">${avgPrice.toFixed(2)}</td>
                          <td className="p-3 text-center font-bold text-base">{costPerf === Infinity ? "∞" : costPerf}</td>
                          <td className="p-3 text-center">
                            <span className={cn("text-xs font-medium px-2 py-0.5 rounded-lg border",
                              costPerf > 1000 ? "bg-success/10 text-success border-success/20" :
                              costPerf > 500 ? "bg-primary/10 text-primary border-primary/20" :
                              costPerf > 200 ? "bg-warning/10 text-warning border-warning/20" :
                              "bg-danger/10 text-danger border-danger/20"
                            )}>{costPerf > 1000 ? "极高" : costPerf > 500 ? "高" : costPerf > 200 ? "中" : "低"}</span>
                          </td>
                        </>}

                        {arenaSubTab === "cheap" && <>
                          <td className="p-3 text-right font-mono text-xs">{m.input_per_m > 0 ? `$${m.input_per_m}` : <span className="text-success">免费</span>}</td>
                          <td className="p-3 text-right font-mono text-xs">{m.output_per_m > 0 ? `$${m.output_per_m}` : <span className="text-success">免费</span>}</td>
                          <td className="p-3 text-center font-medium">{m.arena_score}</td>
                          <td className="p-3 text-center text-xs text-text-faint">{m.context_window >= 1000000 ? (m.context_window/1000000)+"M" : (m.context_window/1000)+"K"}</td>
                        </>}
                      </tr>
                    );
                  });
                })()}
              </tbody>
            </table>
          </div>
          <p className="text-xs text-text-faint">
            {arenaSubTab === "overall" && "按 Arena ELO 排序 · 数据来自公开排行榜 · 点击「更新评测数据」拉取最新分数"}
            {arenaSubTab === "code" && "Code Arena: LMSYS 编程专项 ELO · SWE-bench/Aider/HumanEval: 独立编程评测"}
            {arenaSubTab === "value" && "性价比 = Arena ELO / 平均价格($/M) · 数值越高越划算"}
            {arenaSubTab === "cheap" && "按平均价格升序 · 免费模型排最前"}
          </p>
        </div>
      )}

      {/* ===== 订阅计划（可切换当前档位）===== */}
      {tab === "plans" && (() => {
        // 每个 provider 的所有档位
        const PROVIDER_PLANS: { provider_id: string; provider_name: string; plans: { id: string; name: string; usd: number; mode: string; desc: string }[] }[] = [
          {
            provider_id: "anthropic", provider_name: "Anthropic (Claude)",
            plans: [
              { id: "api", name: "API 按量付费", usd: 0, mode: "api", desc: "按 token 精确计费，适合零散用" },
              { id: "pro", name: "Claude Pro", usd: 20, mode: "subscription", desc: "5 小时窗口约 50 条 Sonnet 消息" },
              { id: "max5", name: "Claude Max 5×", usd: 100, mode: "subscription", desc: "Pro 5 倍额度，适合中重度用户" },
              { id: "max20", name: "Claude Max 20×", usd: 200, mode: "subscription", desc: "Pro 20 倍额度，重度开发者首选" },
              { id: "team", name: "Claude Team", usd: 25, mode: "subscription", desc: "团队版，每人/月" },
            ],
          },
          {
            provider_id: "openai", provider_name: "OpenAI (ChatGPT)",
            plans: [
              { id: "api", name: "API 按量付费", usd: 0, mode: "api", desc: "按 token 精确计费" },
              { id: "plus", name: "ChatGPT Plus", usd: 20, mode: "subscription", desc: "GPT-5.4 + DALL-E + 80次/3h" },
              { id: "pro", name: "ChatGPT Pro", usd: 200, mode: "subscription", desc: "o3 无限 + Sora 视频" },
              { id: "team", name: "ChatGPT Team", usd: 25, mode: "subscription", desc: "团队版，每人/月" },
            ],
          },
          {
            provider_id: "google", provider_name: "Google (Gemini)",
            plans: [
              { id: "api", name: "API 按量付费", usd: 0, mode: "api", desc: "Google AI Studio" },
              { id: "advanced", name: "Gemini Advanced", usd: 19.99, mode: "subscription", desc: "Gemini 3.1 Pro + 2TB 存储" },
              { id: "ultra", name: "Gemini AI Ultra", usd: 249.99, mode: "subscription", desc: "Deep Research + Veo 视频" },
            ],
          },
          {
            provider_id: "cursor", provider_name: "Cursor",
            plans: [
              { id: "hobby", name: "免费版", usd: 0, mode: "api", desc: "每月有限次数，适合尝试" },
              { id: "pro", name: "Cursor Pro", usd: 20, mode: "subscription", desc: "500 次快速 + 无限慢速" },
              { id: "business", name: "Cursor Business", usd: 40, mode: "subscription", desc: "团队版，每人/月" },
            ],
          },
          {
            provider_id: "copilot", provider_name: "GitHub Copilot",
            plans: [
              { id: "free", name: "免费版", usd: 0, mode: "api", desc: "学生/开源维护者" },
              { id: "pro", name: "Copilot Pro", usd: 10, mode: "subscription", desc: "基础订阅" },
              { id: "pro_plus", name: "Copilot Pro+", usd: 39, mode: "subscription", desc: "前沿模型访问" },
              { id: "business", name: "Copilot Business", usd: 19, mode: "subscription", desc: "团队版，每人/月" },
            ],
          },
          {
            provider_id: "xai", provider_name: "xAI (Grok)",
            plans: [
              { id: "api", name: "API 按量付费", usd: 0, mode: "api", desc: "按 token 付费" },
              { id: "plus", name: "SuperGrok", usd: 30, mode: "subscription", desc: "Grok 4 标准订阅" },
              { id: "heavy", name: "SuperGrok Heavy", usd: 300, mode: "subscription", desc: "Grok 4 Heavy + 优先访问" },
            ],
          },
        ];

        async function switchPlan(providerId: string, plan: { usd: number; mode: string }) {
          try {
            await invoke("set_account_mode", { providerId, mode: plan.mode, subscriptionMonthlyUsd: plan.usd });
            const updated = await invoke<typeof accountModes>("get_account_modes");
            setAccountModes(updated);
            // Refresh ROI
            invoke<RoiData>("get_subscription_roi").then(setRoiData);
          } catch (e) { console.error(e); }
        }

        return (
          <div className="space-y-6">
            <div className="bg-primary/5 rounded-xl p-4 border border-primary/20 text-[12px] text-text-muted leading-relaxed">
              <strong className="text-primary">如何使用：</strong>点击任意档位即可标记为你当前使用的订阅。系统会自动把这个 Provider 的所有历史流量重新归类为"订阅虚拟等价"，Dashboard 费用构成立刻更新。
            </div>

            {PROVIDER_PLANS.map(pg => {
              const current = accountModes.find(m => m.provider_id === pg.provider_id);
              const currentUsd = current?.subscription_monthly_usd || 0;
              const currentMode = current?.mode || "api";
              const currentPlanId = pg.plans.find(p => p.mode === currentMode && Math.abs(p.usd - currentUsd) < 0.01)?.id || "api";

              return (
                <div key={pg.provider_id}>
                  <div className="flex items-center gap-2 mb-3">
                    <div className="w-2.5 h-2.5 rounded-full" style={{ backgroundColor: pColor[pg.provider_id] || "#666" }} />
                    <h3 className="text-[14px] font-medium">{pg.provider_name}</h3>
                    {current && currentMode === "subscription" && (
                      <span className="text-[10px] px-2 py-0.5 rounded-full bg-success/10 text-success border border-success/20">
                        当前订阅中
                      </span>
                    )}
                  </div>
                  <div className="grid grid-cols-3 gap-3">
                    {pg.plans.map(p => {
                      const selected = p.id === currentPlanId;
                      return (
                        <button key={p.id}
                          onClick={() => switchPlan(pg.provider_id, p)}
                          className={cn(
                            "text-left p-4 rounded-xl border-2 transition-all",
                            selected
                              ? "border-primary bg-primary/5 shadow-md"
                              : "border-border-light bg-surface-light hover:border-primary/40 hover:shadow"
                          )}>
                          <div className="flex items-start justify-between mb-2">
                            <div className="font-medium text-[13px]">{p.name}</div>
                            {selected && <span className="text-primary text-[11px]">✓ 已选</span>}
                          </div>
                          <div className="text-[20px] font-bold tracking-tight mb-1">
                            {p.usd > 0 ? (
                              <>¥{(p.usd * 7.25).toFixed(0)}<span className="text-[11px] text-text-faint ml-1">/月 · ${p.usd}</span></>
                            ) : <span className="text-success text-[16px]">按量/免费</span>}
                          </div>
                          <p className="text-[11px] text-text-muted leading-relaxed">{p.desc}</p>
                        </button>
                      );
                    })}
                  </div>
                </div>
              );
            })}

            <div className="text-[11px] text-text-faint pt-2 border-t border-border-light">
              💡 切换后历史流量自动按新档位重新归类。未检测到的 Provider 可去「设置 → 账户模式」补充。
            </div>
          </div>
        );
      })()}

      {/* ===== ROI 分析 ===== */}
      {tab === "roi" && (
        <div className="space-y-6">
          {roiData && roiData.results.length > 0 ? (
            <>
              {/* Summary */}
              <div className="grid grid-cols-3 gap-4">
                <div className="bg-surface-light rounded-2xl p-5 border border-border">
                  <div className="text-xs text-text-faint mb-1.5">订阅总费用 (月)</div>
                  <div className="text-2xl font-bold tracking-tight">
                    ¥{roiData.results.reduce((a, r) => a + r.subscription_cny, 0).toFixed(0)}
                  </div>
                </div>
                <div className="bg-surface-light rounded-2xl p-5 border border-border">
                  <div className="text-xs text-text-faint mb-1.5">等价 API 花费 (30天)</div>
                  <div className="text-2xl font-bold tracking-tight">
                    ¥{roiData.results.reduce((a, r) => a + r.api_cost_cny, 0).toFixed(2)}
                  </div>
                </div>
                <div className="bg-surface-light rounded-2xl p-5 border border-border">
                  <div className="text-xs text-text-faint mb-1.5">总节省 / 多花</div>
                  {(() => {
                    const totalSavings = roiData.results.reduce((a, r) => a + r.savings_cny, 0);
                    const positive = totalSavings > 0;
                    return (
                      <div className={cn("text-2xl font-bold tracking-tight", positive ? "text-success" : "text-danger")}>
                        {positive ? "+" : ""}¥{totalSavings.toFixed(2)}
                      </div>
                    );
                  })()}
                </div>
              </div>

              {/* Per-subscription ROI cards */}
              <div className="space-y-4">
                {roiData.results.map((r, i) => {
                  const positive = r.savings_usd > 0;
                  const recIcon = r.recommendation.includes("保留") ? CheckCircle2 :
                                  r.recommendation.includes("取消") ? XCircle : MinusCircle;
                  const RecIcon = recIcon;
                  const recColor = r.recommendation.includes("强烈") ? "text-success" :
                                   r.recommendation.includes("保留") ? "text-primary" :
                                   r.recommendation.includes("取消") ? "text-danger" : "text-warning";

                  return (
                    <div key={i} className="bg-surface-light rounded-2xl p-6 border border-border hover:shadow-lg hover:-translate-y-0.5 transition-all">
                      <div className="flex items-center justify-between mb-5">
                        <div className="flex items-center gap-3">
                          <div className="w-10 h-10 rounded-xl flex items-center justify-center text-white font-bold"
                            style={{ backgroundColor: pColor[r.provider] || "#666" }}>
                            {r.provider_name[0]}
                          </div>
                          <div>
                            <div className="font-medium text-lg">{r.provider_name}</div>
                            <div className="text-xs text-text-faint">{r.plan_name}</div>
                          </div>
                        </div>
                        <div className={cn("flex items-center gap-1.5 text-sm px-3 py-1.5 rounded-full border font-medium",
                          recColor,
                          r.recommendation.includes("强烈") ? "bg-success/10 border-success/20" :
                          r.recommendation.includes("保留") ? "bg-primary/10 border-primary/20" :
                          r.recommendation.includes("取消") ? "bg-danger/10 border-danger/20" :
                          "bg-warning/10 border-warning/20"
                        )}>
                          <RecIcon size={14} />
                          {r.recommendation}
                        </div>
                      </div>

                      {/* ROI percentage highlight */}
                      <div className="flex items-center gap-6 mb-5">
                        <div className={cn("text-5xl font-bold tracking-tighter", positive ? "text-success" : "text-danger")}>
                          {r.roi_percent > 0 ? "+" : ""}{r.roi_percent.toFixed(0)}%
                        </div>
                        <div className="text-sm text-text-muted leading-relaxed">
                          {positive
                            ? `订阅比纯 API 节省了 ¥${r.savings_cny.toFixed(2)}，ROI 为正，值得保留。`
                            : `纯 API 比订阅便宜 ¥${Math.abs(r.savings_cny).toFixed(2)}，建议评估是否需要订阅权益。`}
                        </div>
                      </div>

                      {/* Metrics grid */}
                      <div className="grid grid-cols-4 gap-3">
                        <div className="bg-surface rounded-xl p-3">
                          <div className="text-text-faint text-xs mb-1">订阅费用</div>
                          <div className="font-bold text-lg">¥{r.subscription_cny.toFixed(0)}</div>
                          <div className="text-xs text-text-faint">${r.subscription_usd}/月</div>
                        </div>
                        <div className="bg-surface rounded-xl p-3">
                          <div className="text-text-faint text-xs mb-1">API 等价</div>
                          <div className="font-bold text-lg">¥{r.api_cost_cny.toFixed(2)}</div>
                          <div className="text-xs text-text-faint">${r.api_cost_usd.toFixed(2)}</div>
                        </div>
                        <div className="bg-surface rounded-xl p-3">
                          <div className="text-text-faint text-xs mb-1">30天请求</div>
                          <div className="font-bold text-lg">{r.request_count.toLocaleString()}</div>
                          <div className="text-xs text-text-faint">次</div>
                        </div>
                        <div className="bg-surface rounded-xl p-3">
                          <div className="text-text-faint text-xs mb-1">单次成本</div>
                          <div className="font-bold text-lg">${r.cost_per_request.toFixed(4)}</div>
                          <div className="text-xs text-text-faint">每请求</div>
                        </div>
                      </div>

                      {/* Monthly prediction */}
                      {r.predicted_next_month_usd > 0 && (
                        <div className="mt-3 bg-surface-lighter/50 rounded-xl p-3 border border-border-light">
                          <div className="flex items-center justify-between">
                            <span className="text-[12px] text-text-faint">下月预测 API 花费</span>
                            <div className="flex items-center gap-2">
                              <span className="text-[14px] font-bold">¥{r.predicted_next_month_cny.toFixed(2)}</span>
                              {r.trend_percent !== 0 && (
                                <span className={cn("text-[11px] px-1.5 py-0.5 rounded font-medium flex items-center gap-0.5",
                                  r.trend_percent > 0 ? "bg-danger/10 text-danger" : "bg-success/10 text-success")}>
                                  {r.trend_percent > 0 ? <TrendingUp size={11} /> : <TrendingDown size={11} />}
                                  {r.trend_percent > 0 ? "+" : ""}{r.trend_percent.toFixed(0)}%
                                </span>
                              )}
                            </div>
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>

              <p className="text-xs text-text-faint">
                ROI = (API等价花费 - 订阅费用) / 订阅费用 * 100% · 正值表示订阅更划算 · 基于近 30 天实际使用量计算 · 预测基于近两周趋势
              </p>
            </>
          ) : (
            <div className="bg-surface-light rounded-2xl p-12 border border-border text-center">
              <Calculator size={32} className="mx-auto mb-4 text-text-faint" />
              <p className="text-text-muted">暂无足够数据计算 ROI</p>
              <p className="text-sm text-text-faint mt-1">使用 AI 工具一段时间后自动生成分析</p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
