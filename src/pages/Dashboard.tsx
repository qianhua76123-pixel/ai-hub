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
interface RankingsResult { arena_text: RankedModel[]; arena_code: RankedModel[]; arena_vision: RankedModel[]; artificial_analysis: RankedModel[]; fetched_at: string; errors: string[]; }

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

  useEffect(() => {
    invoke<DetectedProvider[]>("scan_providers").then(setProviders);
    invoke<{ currency_rate: number }>("get_app_info").then(info => setCnyRate(info.currency_rate || 7.2)).catch(() => {});
    invoke<RankingsResult>("fetch_rankings", { aaApiKey: null }).then(r => setTopModels(r.arena_text.slice(0, 5))).catch(() => {});
    invoke<{ requests: number; tokens: number; cost: number }>("get_total_stats").then(setTotalStats).catch(() => {});
    const load = () => {
      invoke<TrafficRecord[]>("get_recent_traffic", { limit: 20 }).then((r) => {
        setTraffic(r);
        const s = new Date(); s.setHours(0, 0, 0, 0);
        const t = r.filter((x) => x.timestamp >= s.getTime());
        setTodayStats({ tasks: t.length, tokens: t.reduce((a, x) => a + x.input_tokens + x.output_tokens, 0), cost: t.reduce((a, x) => a + x.estimated_cost, 0) });
      }).catch(() => {});
      invoke<DailyUsage[]>("get_daily_usage", { days: 7 }).then(setDailyUsage).catch(() => {});
      invoke<RateLimitStatus[]>("get_rate_limit_status").then(setHealthData).catch(() => {});
      invoke<{ monthly_spend_usd: number; monthly_limit_usd: number; percent: number; warning_level: string }>("get_budget_status").then(setBudget).catch(() => {});
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
          { label: "今日 Token", value: formatTokens(todayStats.tokens), sub: "", color: "#f59e0b" },
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
