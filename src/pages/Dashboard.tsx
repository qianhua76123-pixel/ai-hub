import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";
import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer } from "recharts";
import { cn, formatTokens } from "../lib/utils";
import { Trophy, ArrowRight } from "lucide-react";

interface DetectedProvider { id: string; name: string; status: string; detection_detail: string; color: string; }
interface TrafficRecord { id: string; timestamp: number; provider_id: string; model: string; input_tokens: number; output_tokens: number; status: string; estimated_cost: number; }
interface DailyUsage { day: string; tokens: number; }
interface RateLimitStatus { provider_id: string; provider_name: string; status: string; latency_ms: number; error_rate: number; rate_limit_remaining: number | null; warning_level: string; estimated_minutes_left: number | null; last_check: number; }
interface RankedModel { rank: number; name: string; provider: string; score: number; source: string; category: string; votes: number; ci: number; license: string; }
interface RankingsResult { arena_text: RankedModel[]; arena_code: RankedModel[]; arena_vision: RankedModel[]; arena_document?: RankedModel[]; arena_search?: RankedModel[]; arena_image?: RankedModel[]; artificial_analysis: RankedModel[]; fetched_at: string; errors: string[]; }

const tt = { backgroundColor: "var(--color-surface-light)", border: "1px solid var(--color-border)", borderRadius: "10px", color: "var(--color-text)", fontSize: 12, boxShadow: "0 4px 20px rgba(0,0,0,0.1)" };

const PROVIDER_COLORS: Record<string, string> = { Anthropic: "#d97706", OpenAI: "#10b981", Google: "#3b82f6", xAI: "#6366f1", DeepSeek: "#0d9488", Meta: "#0ea5e9" };

export default function Dashboard() {
  const navigate = useNavigate();
  const [providers, setProviders] = useState<DetectedProvider[]>([]);
  const [traffic, setTraffic] = useState<TrafficRecord[]>([]);
  const [dailyUsage, setDailyUsage] = useState<DailyUsage[]>([]);
  const [todayStats, setTodayStats] = useState({ tasks: 0, tokens: 0, cost: 0 });
  const [totalStats, setTotalStats] = useState({ requests: 0, tokens: 0, cost: 0 });
  const [healthData, setHealthData] = useState<RateLimitStatus[]>([]);
  const [cnyRate, setCnyRate] = useState(7.2);
  const [budget, setBudget] = useState<{ monthly_spend_usd: number; monthly_limit_usd: number; percent: number; warning_level: string } | null>(null);
  const [topModels, setTopModels] = useState<RankedModel[]>([]);
  const [cacheSummary, setCacheSummary] = useState<{ today: { input: number; cache_write: number; cache_read: number; output: number }; month: { input: number; cache_write: number; cache_read: number; output: number } } | null>(null);
  const [costBreakdown, setCostBreakdown] = useState<{ api_cost_usd: number; api_requests: number; subscription_virtual_cost_usd: number; subscription_requests: number; subscription_monthly_fee_usd: number; subscription_savings_usd: number; total_actual_usd: number; total_virtual_equivalent_usd: number } | null>(null);

  useEffect(() => {
    invoke<DetectedProvider[]>("scan_providers").then(setProviders);
    invoke<{ currency_rate: number }>("get_app_info").then(info => setCnyRate(info.currency_rate || 7.2)).catch(() => {});
    invoke<RankingsResult>("fetch_rankings", { aaApiKey: null }).then(r => setTopModels(r.arena_text.slice(0, 5))).catch(() => {});
    invoke<{ requests: number; tokens: number; cost: number }>("get_total_stats").then(setTotalStats).catch(() => {});
    const load = () => {
      // 实时流量列表（仅用于显示最近 20 条，不用于统计）
      invoke<TrafficRecord[]>("get_recent_traffic", { limit: 20 }).then(setTraffic).catch(() => {});
      // 今日真实统计（后端全量 SUM，不受 limit 影响）
      invoke<{ requests: number; tokens: number; cost: number }>("get_today_stats")
        .then(s => setTodayStats({ tasks: s.requests, tokens: s.tokens, cost: s.cost }))
        .catch(() => {});
      invoke<DailyUsage[]>("get_daily_usage", { days: 7 }).then(setDailyUsage).catch(() => {});
      invoke<RateLimitStatus[]>("get_rate_limit_status").then(setHealthData).catch(() => {});
      invoke<{ monthly_spend_usd: number; monthly_limit_usd: number; percent: number; warning_level: string }>("get_budget_status").then(setBudget).catch(() => {});
      invoke<{ today: { input: number; cache_write: number; cache_read: number; output: number }; month: { input: number; cache_write: number; cache_read: number; output: number } }>("get_cache_summary").then(setCacheSummary).catch(() => {});
      invoke<typeof costBreakdown>("get_cost_breakdown", { days: 30 }).then(setCostBreakdown).catch(() => {});
    };
    load(); const i = setInterval(load, 10000); return () => clearInterval(i);
  }, []);

  function timeAgo(ts: number) {
    const d = Math.floor((Date.now() - ts) / 1000);
    if (d < 10) return "刚刚"; if (d < 60) return d + " 秒前"; if (d < 3600) return Math.floor(d / 60) + " 分钟前";
    if (d < 86400) return Math.floor(d / 3600) + " 小时前"; return Math.floor(d / 86400) + " 天前";
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-[22px] font-semibold tracking-tight text-text">总览</h1>
        <p className="text-[13px] text-text-muted mt-0.5">AI 工具使用情况一览</p>
      </div>

      {/* Budget progress bar */}
      {budget && budget.monthly_limit_usd > 0 && (
        <div className={cn("card px-5 py-3", budget.warning_level === "exceeded" ? "border-danger/30 bg-danger/3" : budget.warning_level === "critical" ? "border-warning/30 bg-warning/3" : "")}>
          <div className="flex items-center justify-between mb-1.5">
            <span className="text-[12px] text-text-muted">本月预算</span>
            <span className={cn("text-[13px] font-medium",
              budget.warning_level === "exceeded" ? "text-danger" : budget.warning_level === "critical" ? "text-warning" : "text-text")}>
              ¥{(budget.monthly_spend_usd * cnyRate).toFixed(2)} / ¥{(budget.monthly_limit_usd * cnyRate).toFixed(0)}
              <span className="text-text-faint ml-1.5 text-[11px]">({budget.percent.toFixed(0)}%)</span>
            </span>
          </div>
          <div className="w-full h-2 bg-surface-lighter rounded-full overflow-hidden">
            <div className={cn("h-full rounded-full transition-all",
              budget.percent >= 100 ? "bg-danger" : budget.percent >= 90 ? "bg-warning" : budget.percent >= 70 ? "bg-warning" : "bg-primary"
            )} style={{ width: `${Math.min(100, budget.percent)}%` }} />
          </div>
        </div>
      )}

      {/* Stats — premium cards */}
      <div className="grid grid-cols-4 gap-4">
        {[
          { label: "已接入工具", value: String(providers.length), sub: "个", color: "#0d9488" },
          { label: "今日请求", value: todayStats.tasks.toLocaleString(), sub: "次", color: "#0ea5e9" },
          { label: "今日新消耗 Token", value: formatTokens(todayStats.tokens), sub: "", color: "#f59e0b" },
          { label: "今日费用", value: "¥" + (todayStats.cost * cnyRate).toFixed(2), sub: "", color: "#ef4444" },
        ].map((s) => (
          <div key={s.label} className="card p-5 glow-teal">
            <div className="text-[12px] text-text-muted mb-2">{s.label}</div>
            <div className="stat-value text-[28px]" style={{ color: s.color }}>
              {s.value}<span className="text-[14px] text-text-faint ml-0.5">{s.sub}</span>
            </div>
          </div>
        ))}
      </div>

      {/* Cost breakdown: API actual vs Subscription virtual */}
      {costBreakdown && (costBreakdown.api_cost_usd > 0 || costBreakdown.subscription_virtual_cost_usd > 0) && (() => {
        // 合理 API 等价：假设用户改用 API 时会做 prompt 优化，减少 70% 的 cache_read 重复
        // 虚拟费用 ≈ 99% 来自 cache_read，乘以 0.3 代表"做了合理优化后的真实账单"
        const reasonableApiUsd = costBreakdown.subscription_virtual_cost_usd * 0.3;
        const reasonableSavings = reasonableApiUsd - costBreakdown.subscription_monthly_fee_usd;
        return (
        <div className="card p-5">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-[13px] font-medium text-text-muted">近 30 天费用构成</h2>
            <span className="text-[11px] text-text-faint">实际付费 vs 订阅等价</span>
          </div>
          <div className="grid grid-cols-3 gap-4">
            <div className="bg-primary/5 rounded-[10px] p-3 border border-primary/15">
              <div className="text-[11px] text-text-muted mb-1">API 实际付费</div>
              <div className="text-[20px] font-semibold text-primary">
                ¥{(costBreakdown.api_cost_usd * cnyRate).toFixed(2)}
              </div>
              <div className="text-[10px] text-text-faint mt-0.5">{costBreakdown.api_requests.toLocaleString()} 次请求</div>
            </div>
            <div className="bg-success/5 rounded-[10px] p-3 border border-success/15">
              <div className="text-[11px] text-text-muted mb-1">订阅月费（真实账单）</div>
              <div className="text-[20px] font-semibold text-success">
                ¥{(costBreakdown.subscription_monthly_fee_usd * cnyRate).toFixed(0)}
              </div>
              <div className="text-[10px] text-text-faint mt-0.5">{costBreakdown.subscription_requests.toLocaleString()} 次请求</div>
            </div>
            <div className="bg-warning/5 rounded-[10px] p-3 border border-warning/15">
              <div className="text-[11px] text-text-muted mb-1 flex items-center gap-1">
                同量 API 等价
                <span className="text-[9px] text-text-faint px-1 py-[1px] rounded bg-warning/10">优化后</span>
              </div>
              <div className="text-[20px] font-semibold text-warning">
                ¥{(reasonableApiUsd * cnyRate).toFixed(0)}
              </div>
              <div className="text-[10px] text-text-faint mt-0.5">
                理论上限 ¥{(costBreakdown.subscription_virtual_cost_usd * cnyRate).toFixed(0)}
              </div>
            </div>
          </div>

          {/* 关键说明：避免让用户被"订阅省1.4万"数字误导 */}
          <div className="mt-3 bg-surface-lighter/60 rounded-[8px] p-3 text-[11px] text-text-muted leading-relaxed">
            <div className="font-medium text-text mb-1">💡 如何解读这些数字</div>
            <div>
              订阅 ¥{(costBreakdown.subscription_monthly_fee_usd * cnyRate).toFixed(0)} 看起来比"按 API 算等价 ¥{(costBreakdown.subscription_virtual_cost_usd * cnyRate).toFixed(0)}"便宜十几倍 —— 这并非 bug 也不是慈善：
              <span className="block mt-1">
                · Claude Code 每次请求都重发 60 万+ 上下文，其中 99% 是<strong className="text-text-muted">缓存复用</strong>（cache_read）。
              </span>
              <span className="block">
                · Anthropic 对 cache_read 只收 <strong className="text-text-muted">10% 价</strong>，因为他们的 GPU 处理 cache hit 成本也只有 ~10%。
              </span>
              <span className="block">
                · 真 API 用户会做 prompt 优化，账单约 <strong className="text-text-muted">¥{(reasonableApiUsd * cnyRate).toFixed(0)}/月</strong>（理论上限的 30%）。
              </span>
              <span className="block mt-1">
                · 订阅还有 <strong className="text-text-muted">5h 窗口额度限制</strong>，不是无限用。但对重度 Claude Code 用户，订阅确实比 API 更省 —— 合理节省 <strong className="text-success">¥{(reasonableSavings * cnyRate).toFixed(0)}/月</strong>。
              </span>
            </div>
          </div>

          <div className="mt-3 pt-3 border-t border-border-light text-[11px] text-text-faint">
            总实际支出 <strong className="text-text-muted">¥{(costBreakdown.total_actual_usd * cnyRate).toFixed(2)}</strong>
            {" "}· 按 API 全量计费等价 <strong className="text-text-muted">¥{(costBreakdown.total_virtual_equivalent_usd * cnyRate).toFixed(2)}</strong>
            {costBreakdown.subscription_monthly_fee_usd === 0 && (
              <span className="ml-2 text-warning">· 未设置订阅？到 <strong>订阅 → 订阅计划</strong> 标记</span>
            )}
          </div>
        </div>
        );
      })()}

      {/* Token breakdown — show cache efficiency */}
      {cacheSummary && (cacheSummary.today.cache_read > 0 || cacheSummary.today.cache_write > 0) && (
        <div className="card p-5">
          <div className="flex items-center justify-between mb-3">
            <h2 className="text-[13px] font-medium text-text-muted">今日 Token 结构</h2>
            {cacheSummary.today.cache_read > 0 && (
              <span className="text-[11px] text-success">
                缓存复用率 {(cacheSummary.today.cache_read / (cacheSummary.today.input + cacheSummary.today.cache_write + cacheSummary.today.cache_read) * 100).toFixed(1)}%
              </span>
            )}
          </div>
          <div className="grid grid-cols-4 gap-3">
            <div>
              <div className="text-[11px] text-text-faint mb-1">新输入</div>
              <div className="text-[18px] font-semibold">{formatTokens(cacheSummary.today.input)}</div>
              <div className="text-[10px] text-text-faint mt-0.5">用户新消息</div>
            </div>
            <div>
              <div className="text-[11px] text-text-faint mb-1">缓存写入</div>
              <div className="text-[18px] font-semibold text-warning">{formatTokens(cacheSummary.today.cache_write)}</div>
              <div className="text-[10px] text-text-faint mt-0.5">~1.25x 价</div>
            </div>
            <div>
              <div className="text-[11px] text-text-faint mb-1">缓存读取</div>
              <div className="text-[18px] font-semibold text-success">{formatTokens(cacheSummary.today.cache_read)}</div>
              <div className="text-[10px] text-text-faint mt-0.5">~10% 价 (省 90%)</div>
            </div>
            <div>
              <div className="text-[11px] text-text-faint mb-1">输出</div>
              <div className="text-[18px] font-semibold text-primary">{formatTokens(cacheSummary.today.output)}</div>
              <div className="text-[10px] text-text-faint mt-0.5">AI 回复</div>
            </div>
          </div>
          {cacheSummary.today.cache_read > 0 && (
            <div className="mt-3 pt-3 border-t border-border-light text-[11px] text-text-faint">
              若没有缓存，相同上下文会按 <strong className="text-text-muted">{formatTokens(cacheSummary.today.cache_read)}</strong> tokens × 全价计费。
              缓存让你实际只付了约 <strong className="text-success">{(cacheSummary.today.cache_read * 0.1 / 1000).toFixed(1)}K</strong> 等价的费用。
            </div>
          )}
        </div>
      )}

      {/* Cumulative — subtle row */}
      <div className="flex items-center gap-6 px-1">
        {[
          { label: "累计请求", value: totalStats.requests.toLocaleString() + " 次" },
          { label: "累计 Token", value: formatTokens(totalStats.tokens) },
          { label: "累计费用", value: "¥" + (totalStats.cost * cnyRate).toFixed(2) },
        ].map((s) => (
          <div key={s.label} className="flex items-center gap-2">
            <span className="text-[12px] text-text-faint">{s.label}</span>
            <span className="text-[13px] font-medium text-text-muted">{s.value}</span>
          </div>
        ))}
      </div>

      {/* Chart + Traffic */}
      <div className="grid grid-cols-5 gap-4">
        <div className="col-span-3 card p-5">
          <h2 className="text-[13px] font-medium text-text-muted mb-4">近 7 天 Token 用量</h2>
          {dailyUsage.length > 0 ? (
            <ResponsiveContainer width="100%" height={200}>
              <AreaChart data={dailyUsage}>
                <defs>
                  <linearGradient id="tg" x1="0" y1="0" x2="0" y2="1">
                    <stop offset="0%" stopColor="#0d9488" stopOpacity={0.12} />
                    <stop offset="100%" stopColor="#0d9488" stopOpacity={0.01} />
                  </linearGradient>
                </defs>
                <XAxis dataKey="day" stroke="#e2e5eb" fontSize={11} tickLine={false} axisLine={false} tick={{ fill: "#9ca3af" }} />
                <YAxis stroke="#e2e5eb" fontSize={11} tickLine={false} axisLine={false} tick={{ fill: "#9ca3af" }} tickFormatter={formatTokens} />
                <Tooltip contentStyle={tt} formatter={(v) => [formatTokens(Number(v)), "Token"]} />
                <Area type="monotone" dataKey="tokens" stroke="#0d9488" fill="url(#tg)" strokeWidth={2} dot={false} />
              </AreaChart>
            </ResponsiveContainer>
          ) : <div className="h-48 flex items-center justify-center text-[13px] text-text-faint">暂无数据</div>}
        </div>

        <div className="col-span-2 card p-5">
          <h2 className="text-[13px] font-medium text-text-muted mb-4">实时流量</h2>
          {traffic.length > 0 ? (
            <div className="space-y-0.5">
              {traffic.slice(0, 7).map((t) => (
                <div key={t.id} className="flex items-center justify-between py-1.5 px-1 rounded-lg hover:bg-surface-lighter transition-colors">
                  <div className="flex items-center gap-2 min-w-0">
                    <div className={cn("w-[6px] h-[6px] rounded-full shrink-0",
                      t.status === "success" ? "bg-success" : t.status === "rate_limited" ? "bg-warning" : "bg-danger")} />
                    <span className="text-[13px] text-text truncate">{t.model}</span>
                  </div>
                  <div className="flex items-center gap-2.5 shrink-0">
                    <span className="text-[12px] text-primary font-medium">{formatTokens(t.input_tokens + t.output_tokens)}</span>
                    <span className="text-[11px] text-text-faint w-16 text-right">{timeAgo(t.timestamp)}</span>
                  </div>
                </div>
              ))}
            </div>
          ) : <div className="h-40 flex items-center justify-center text-[13px] text-text-faint">等待数据</div>}
        </div>
      </div>

      {/* Model Rankings Card */}
      {topModels.length > 0 && (
        <div className="card p-5">
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2">
              <Trophy size={14} className="text-primary" />
              <h2 className="text-[13px] font-medium text-text-muted">模型排行 · Arena 综合</h2>
            </div>
            <button onClick={() => navigate("/rankings")} className="flex items-center gap-1 text-[11px] text-primary hover:underline">
              查看完整排行 <ArrowRight size={11} />
            </button>
          </div>
          <div className="space-y-1">
            {topModels.map((m, i) => {
              const pColor = PROVIDER_COLORS[m.provider] || "#6b7280";
              const maxS = topModels[0]?.score || 1;
              const barW = Math.max(10, (m.score / maxS) * 100);
              return (
                <div key={m.name} className="flex items-center gap-3 py-2 px-2 rounded-lg hover:bg-surface-lighter transition-colors">
                  <span className="text-[15px] w-6 text-center shrink-0">
                    {i === 0 ? "🥇" : i === 1 ? "🥈" : i === 2 ? "🥉" : <span className="text-[13px] text-text-faint">{m.rank}</span>}
                  </span>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="text-[13px] font-medium text-text truncate">{m.name}</span>
                      <div className="w-[5px] h-[5px] rounded-full shrink-0" style={{ backgroundColor: pColor }} />
                      <span className="text-[11px] text-text-faint">{m.provider}</span>
                    </div>
                    <div className="w-full h-1 bg-surface-lighter rounded-full overflow-hidden">
                      <div className="h-full rounded-full transition-all" style={{ width: `${barW}%`, backgroundColor: pColor, opacity: 0.4 }} />
                    </div>
                  </div>
                  <span className="text-[13px] font-semibold tabular-nums text-text-muted shrink-0">{m.score}</span>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Provider Health */}
      {healthData.length > 0 && (
        <div>
          <h2 className="text-[13px] font-medium text-text-muted mb-3">服务状态</h2>
          {/* Rate limit warning */}
          {healthData.some(h => h.warning_level === "critical") && (
            <div className="bg-danger/5 border border-danger/20 rounded-[10px] px-4 py-2.5 mb-3 flex items-center gap-2">
              <div className="w-2 h-2 rounded-full bg-danger animate-pulse" />
              <span className="text-[12px] text-danger font-medium">
                {healthData.filter(h => h.warning_level === "critical").map(h => h.provider_name).join(", ")} 即将达到速率限制
              </span>
            </div>
          )}
          <div className="grid grid-cols-4 gap-3">
            {healthData.map((h) => {
              const statusColor = h.status === "healthy" ? "bg-success" : h.status === "degraded" ? "bg-warning" : "bg-danger";
              const warnBorder = h.warning_level === "critical" ? "border-danger/30" : h.warning_level === "warning" ? "border-warning/30" : "border-border-light";

              return (
                <div key={h.provider_id} className={cn("card px-4 py-3 border", warnBorder)}>
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-[13px] font-medium">{h.provider_name}</span>
                    <div className={cn("w-2 h-2 rounded-full", statusColor)} />
                  </div>
                  <div className="space-y-1.5">
                    <div className="flex justify-between text-[11px]">
                      <span className="text-text-faint">延迟</span>
                      <span className={cn("font-medium", h.latency_ms > 5000 ? "text-warning" : "text-text-muted")}>{h.latency_ms > 0 ? `${h.latency_ms}ms` : "-"}</span>
                    </div>
                    {h.rate_limit_remaining !== null && (
                      <div>
                        <div className="flex justify-between text-[11px] mb-1">
                          <span className="text-text-faint">配额</span>
                          <span className={cn("font-medium",
                            h.warning_level === "critical" ? "text-danger" :
                            h.warning_level === "warning" ? "text-warning" : "text-text-muted"
                          )}>剩余 {h.rate_limit_remaining}</span>
                        </div>
                        <div className="w-full h-1.5 bg-surface-lighter rounded-full overflow-hidden">
                          <div className={cn("h-full rounded-full transition-all",
                            h.warning_level === "critical" ? "bg-danger" :
                            h.warning_level === "warning" ? "bg-warning" : "bg-success"
                          )} style={{ width: `${Math.min(100, Math.max(5, (h.rate_limit_remaining ?? 0) * 2))}%` }} />
                        </div>
                      </div>
                    )}
                    {h.error_rate > 0.01 && (
                      <div className="flex justify-between text-[11px]">
                        <span className="text-text-faint">错误率</span>
                        <span className="text-danger font-medium">{(h.error_rate * 100).toFixed(1)}%</span>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Providers */}
      {providers.length > 0 && (
        <div>
          <h2 className="text-[13px] font-medium text-text-muted mb-3">已检测工具</h2>
          <div className="grid grid-cols-3 gap-3">
            {providers.map((p) => (
              <div key={p.id} className="card px-4 py-3 flex items-center gap-3">
                <div className="w-8 h-8 rounded-[8px] flex items-center justify-center text-white text-[12px] font-semibold shrink-0"
                  style={{ backgroundColor: p.color }}>{p.name[0]}</div>
                <div className="flex-1 min-w-0">
                  <div className="text-[13px] font-medium text-text">{p.name}</div>
                  <div className="text-[11px] text-text-faint truncate">{p.detection_detail}</div>
                </div>
                <div className={cn("w-2 h-2 rounded-full shrink-0", p.status === "connected" ? "bg-success" : "bg-warning")} />
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
