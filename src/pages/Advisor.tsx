import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Sparkles, AlertTriangle, TrendingDown, Plus, Calculator, Loader2, ChevronRight, X, Info } from "lucide-react";
import { cn } from "../lib/utils";

interface UserSubscription {
  id: string;
  provider_id: string;
  provider_name: string;
  plan_name: string;
  monthly_usd: number;
  category: string;
  billing_day: number;
  started_at: number;
}

interface Recommendation {
  kind: string;
  severity: string;
  title: string;
  description: string;
  monthly_savings_usd: number;
  affected_subscriptions: string[];
  action: string;
  suggested_replacement: string | null;
}

interface StackCostEstimate {
  total_monthly_usd: number;
  total_yearly_usd: number;
  subscription_count: number;
  breakdown: { plan_name: string; provider_name: string; monthly_usd: number; yearly_usd: number; percent_of_total: number }[];
  savings_if_optimized_usd: number;
}

const categoryOptions = [
  { value: "chat", label: "聊天助手" },
  { value: "coding_ide", label: "IDE 编程" },
  { value: "coding_cli", label: "CLI 编程" },
  { value: "image", label: "图像生成" },
  { value: "other", label: "其他" },
];

const severityColor: Record<string, string> = {
  high: "bg-danger/10 text-danger border-danger/30",
  medium: "bg-warning/10 text-warning border-warning/30",
  low: "bg-primary/10 text-primary border-primary/30",
};

const pColor: Record<string, string> = {
  anthropic: "#d97706", openai: "#10a37f", google: "#4285f4", cursor: "#00d4aa", copilot: "#6e40c9",
  perplexity: "#20808d", midjourney: "#ff00aa", deepseek: "#4d6bfe",
};

export default function Advisor() {
  const [subs, setSubs] = useState<UserSubscription[]>([]);
  const [recs, setRecs] = useState<Recommendation[]>([]);
  const [estimate, setEstimate] = useState<StackCostEstimate | null>(null);
  const [cnyRate, setCnyRate] = useState(7.2);
  const [showAdd, setShowAdd] = useState(false);
  const [loading, setLoading] = useState(false);

  const [providerName, setProviderName] = useState("");
  const [providerId, setProviderId] = useState("");
  const [planName, setPlanName] = useState("");
  const [monthlyUsd, setMonthlyUsd] = useState("");
  const [category, setCategory] = useState("chat");
  const [billingDay] = useState("1");

  function loadAll() {
    invoke<UserSubscription[]>("get_user_subscriptions").then(setSubs);
    invoke<Recommendation[]>("get_subscription_recommendations").then(setRecs);
    invoke<StackCostEstimate>("get_stack_cost_estimate").then(setEstimate);
  }

  useEffect(() => {
    loadAll();
    invoke<{ currency_rate: number }>("get_app_info").then(info => setCnyRate(info.currency_rate || 7.2)).catch(() => {});
  }, []);

  async function handleAdd() {
    if (!planName.trim() || !monthlyUsd) return;
    setLoading(true);
    try {
      await invoke("add_user_subscription", {
        providerId: providerId || planName.toLowerCase().replace(/\s+/g, "-"),
        providerName: providerName || planName,
        planName,
        monthlyUsd: parseFloat(monthlyUsd),
        category,
        billingDay: parseInt(billingDay) || 1,
      });
      setPlanName(""); setMonthlyUsd(""); setProviderName(""); setProviderId("");
      setShowAdd(false);
      loadAll();
    } catch (e) { console.error(e); }
    setLoading(false);
  }

  async function handleDelete(id: string) {
    await invoke("delete_user_subscription", { id });
    loadAll();
  }

  const totalMonthly = estimate?.total_monthly_usd ?? 0;
  const totalSavings = estimate?.savings_if_optimized_usd ?? 0;
  const savingsPercent = totalMonthly > 0 ? (totalSavings / totalMonthly) * 100 : 0;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-[22px] font-semibold tracking-tight">订阅顾问</h1>
          <p className="text-[13px] text-text-muted mt-0.5">检测重叠订阅，发现节省空间</p>
        </div>
        <button onClick={() => setShowAdd(!showAdd)}
          className={cn("flex items-center gap-2 px-4 py-2 rounded-[8px] text-[13px] font-medium transition-all",
            showAdd ? "bg-surface-lighter text-text-muted border border-border" : "bg-primary text-white hover:bg-primary-dark")}>
          <Plus size={15} /> 添加订阅
        </button>
      </div>

      {/* Cost summary cards */}
      {estimate && estimate.subscription_count > 0 && (
        <div className="grid grid-cols-4 gap-3">
          <div className="card p-4">
            <div className="text-[11px] text-text-muted mb-1">订阅数</div>
            <div className="text-[22px] font-semibold">{estimate.subscription_count}</div>
          </div>
          <div className="card p-4">
            <div className="text-[11px] text-text-muted mb-1">月支出</div>
            <div className="text-[22px] font-semibold">¥{(totalMonthly * cnyRate).toFixed(0)}</div>
            <div className="text-[11px] text-text-faint">${totalMonthly.toFixed(0)}/月</div>
          </div>
          <div className="card p-4">
            <div className="text-[11px] text-text-muted mb-1">年支出</div>
            <div className="text-[22px] font-semibold">¥{(estimate.total_yearly_usd * cnyRate).toFixed(0)}</div>
            <div className="text-[11px] text-text-faint">${estimate.total_yearly_usd.toFixed(0)}/年</div>
          </div>
          <div className={cn("card p-4", totalSavings > 0 ? "border-success/30 bg-success/3" : "")}>
            <div className="text-[11px] text-text-muted mb-1">可节省</div>
            <div className={cn("text-[22px] font-semibold", totalSavings > 0 ? "text-success" : "text-text-faint")}>
              ¥{(totalSavings * cnyRate).toFixed(0)}
            </div>
            <div className="text-[11px] text-text-faint">月省 {savingsPercent.toFixed(0)}%</div>
          </div>
        </div>
      )}

      {/* Add form */}
      {showAdd && (
        <div className="card p-5 space-y-3">
          <h3 className="text-[14px] font-medium">添加订阅</h3>
          <div className="grid grid-cols-2 gap-3">
            <input value={planName} onChange={(e) => setPlanName(e.target.value)} placeholder="计划名称（如 ChatGPT Plus）"
              className="bg-surface border border-border rounded-[8px] px-3 py-2 text-[13px] focus:outline-none focus:border-primary" />
            <input value={providerName} onChange={(e) => setProviderName(e.target.value)} placeholder="提供商（如 OpenAI）"
              className="bg-surface border border-border rounded-[8px] px-3 py-2 text-[13px] focus:outline-none focus:border-primary" />
            <input value={monthlyUsd} onChange={(e) => setMonthlyUsd(e.target.value)} placeholder="月费 USD" type="number" step="1"
              className="bg-surface border border-border rounded-[8px] px-3 py-2 text-[13px] focus:outline-none focus:border-primary" />
            <select value={category} onChange={(e) => setCategory(e.target.value)}
              className="bg-surface border border-border rounded-[8px] px-3 py-2 text-[13px] focus:outline-none focus:border-primary">
              {categoryOptions.map(o => <option key={o.value} value={o.value}>{o.label}</option>)}
            </select>
          </div>
          <div className="flex justify-end gap-2">
            <button onClick={() => setShowAdd(false)} className="px-4 py-1.5 rounded-[6px] text-[13px] text-text-muted hover:bg-surface-lighter">取消</button>
            <button onClick={handleAdd} disabled={loading || !planName.trim() || !monthlyUsd}
              className="flex items-center gap-2 px-5 py-1.5 bg-primary hover:bg-primary-dark rounded-[6px] text-[13px] text-white font-medium disabled:opacity-40">
              {loading ? <Loader2 size={13} className="animate-spin" /> : <Plus size={13} />} 添加
            </button>
          </div>
        </div>
      )}

      {/* Recommendations */}
      {recs.length > 0 && (
        <div>
          <div className="flex items-center gap-2 mb-3">
            <Sparkles size={14} className="text-primary" />
            <h2 className="text-[13px] font-medium text-text-muted">节省建议 ({recs.length})</h2>
          </div>
          <div className="space-y-3">
            {recs.map((r, i) => {
              const Icon = r.severity === "high" ? AlertTriangle : r.kind === "underused" ? TrendingDown : Info;
              return (
                <div key={i} className={cn("card p-4 border-l-4", r.severity === "high" ? "border-l-danger" : r.severity === "medium" ? "border-l-warning" : "border-l-primary")}>
                  <div className="flex items-start gap-3">
                    <Icon size={16} className={cn("mt-0.5 shrink-0",
                      r.severity === "high" ? "text-danger" : r.severity === "medium" ? "text-warning" : "text-primary")} />
                    <div className="flex-1 min-w-0">
                      <div className="flex items-start justify-between gap-3 mb-1">
                        <h3 className="text-[14px] font-medium">{r.title}</h3>
                        <span className={cn("text-[11px] px-2 py-0.5 rounded-full border shrink-0", severityColor[r.severity])}>
                          月省 ¥{(r.monthly_savings_usd * cnyRate).toFixed(0)}
                        </span>
                      </div>
                      <p className="text-[12px] text-text-muted leading-relaxed">{r.description}</p>
                      {r.suggested_replacement && (
                        <div className="mt-2 flex items-center gap-1.5 text-[11px] text-primary">
                          <ChevronRight size={11} />
                          建议改用: <strong>{r.suggested_replacement}</strong>
                        </div>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Subscription list */}
      {subs.length > 0 ? (
        <div>
          <h2 className="text-[13px] font-medium text-text-muted mb-3">已录入订阅 ({subs.length})</h2>
          <div className="card overflow-hidden">
            <table className="w-full text-[13px]">
              <thead>
                <tr className="text-text-faint text-[11px] border-b border-border-light bg-surface-lighter">
                  <th className="text-left px-4 py-2.5 font-medium">计划</th>
                  <th className="text-left px-4 py-2.5 font-medium">类型</th>
                  <th className="text-right px-4 py-2.5 font-medium">月费</th>
                  <th className="text-right px-4 py-2.5 font-medium">年费</th>
                  <th className="text-right px-4 py-2.5 font-medium">占比</th>
                  <th className="w-10"></th>
                </tr>
              </thead>
              <tbody>
                {estimate?.breakdown.map((item, i) => {
                  const sub = subs[i];
                  if (!sub) return null;
                  return (
                    <tr key={sub.id} className="border-b border-border-light/50 hover:bg-surface-lighter/50">
                      <td className="px-4 py-2.5">
                        <span className="inline-block w-2 h-2 rounded-full mr-2 align-middle" style={{ backgroundColor: pColor[sub.provider_id] || "#666" }} />
                        <span className="font-medium">{item.plan_name}</span>
                        <span className="text-text-faint ml-2 text-[11px]">{item.provider_name}</span>
                      </td>
                      <td className="px-4 py-2.5 text-text-muted text-[12px]">
                        {categoryOptions.find(o => o.value === sub.category)?.label || sub.category}
                      </td>
                      <td className="text-right px-4 py-2.5 font-medium">¥{(item.monthly_usd * cnyRate).toFixed(0)}</td>
                      <td className="text-right px-4 py-2.5 text-text-muted">¥{(item.yearly_usd * cnyRate).toFixed(0)}</td>
                      <td className="text-right px-4 py-2.5 text-text-faint">{item.percent_of_total.toFixed(0)}%</td>
                      <td className="px-2">
                        <button onClick={() => handleDelete(sub.id)} className="text-text-faint hover:text-danger p-1">
                          <X size={13} />
                        </button>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      ) : !showAdd && (
        <div className="card p-12 text-center">
          <Calculator size={28} className="mx-auto mb-3 text-text-faint" />
          <p className="text-[14px] text-text-muted mb-1">还没有录入任何订阅</p>
          <p className="text-[12px] text-text-faint mb-4">添加你的 AI 订阅，系统会自动检测重叠和浪费，给出省钱建议</p>
          <button onClick={() => setShowAdd(true)}
            className="inline-flex items-center gap-2 px-4 py-2 bg-primary text-white rounded-[6px] text-[13px] font-medium hover:bg-primary-dark">
            <Plus size={13} /> 添加第一个订阅
          </button>
        </div>
      )}
    </div>
  );
}
