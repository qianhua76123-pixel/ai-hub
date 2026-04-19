import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNavigate } from "react-router-dom";
import { Sparkles, AlertTriangle, TrendingDown, TrendingUp, Calculator, ChevronRight, Info, Settings, Zap } from "lucide-react";
import { cn } from "../lib/utils";

interface AdvisorSubItem {
  provider_id: string;
  provider_name: string;
  mode: string;
  monthly_usd: number;
  category: string;
  monthly_requests: number;
  virtual_api_cost_usd: number;
  reasonable_api_cost_usd: number;
  utilization: string;  // high | normal | low | unused
}

interface Recommendation {
  kind: string;
  severity: string;
  title: string;
  description: string;
  monthly_savings_usd: number;
  action: string;
  suggested_replacement: string | null;
}

interface AdvisorResult {
  total_monthly_usd: number;
  total_yearly_usd: number;
  subscription_count: number;
  api_only_count: number;
  items: AdvisorSubItem[];
  recommendations: Recommendation[];
  total_savings_usd: number;
}

const categoryLabel: Record<string, string> = {
  chat: "聊天/通用",
  coding_ide: "IDE 编程",
  coding_cli: "CLI 编程",
  image: "图像生成",
  other: "其他",
};

const utilizationConfig: Record<string, { label: string; color: string; bg: string }> = {
  high:   { label: "高强度", color: "text-success", bg: "bg-success/10" },
  normal: { label: "正常",   color: "text-primary", bg: "bg-primary/10" },
  low:    { label: "低利用", color: "text-warning", bg: "bg-warning/10" },
  unused: { label: "几乎未用", color: "text-danger", bg: "bg-danger/10" },
};

const pColor: Record<string, string> = {
  anthropic: "#d97706", openai: "#10a37f", google: "#4285f4",
  cursor: "#00d4aa", copilot: "#6e40c9", xai: "#6366f1",
};

export default function Advisor() {
  const navigate = useNavigate();
  const [result, setResult] = useState<AdvisorResult | null>(null);
  const [cnyRate, setCnyRate] = useState(7.2);

  function load() {
    invoke<AdvisorResult>("get_advisor_analysis").then(setResult).catch(console.error);
  }

  useEffect(() => {
    load();
    invoke<{ currency_rate: number }>("get_app_info").then(info => setCnyRate(info.currency_rate || 7.2)).catch(() => {});
  }, []);

  const subs = result?.items.filter(i => i.mode !== "api") || [];

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-[22px] font-semibold tracking-tight">订阅顾问</h1>
          <p className="text-[13px] text-text-muted mt-0.5">分析你的订阅组合，检测重叠和利用率问题</p>
        </div>
        <button onClick={() => navigate("/billing")}
          className="flex items-center gap-1.5 px-3.5 py-2 rounded-[8px] text-[13px] text-primary bg-primary/8 hover:bg-primary/12 font-medium transition-colors">
          <Settings size={13} /> 去订阅页调整档位
        </button>
      </div>

      <div className="bg-primary/5 rounded-xl p-4 border border-primary/20 text-[12px] text-text-muted leading-relaxed flex items-start gap-2">
        <Info size={14} className="text-primary shrink-0 mt-0.5" />
        <div>
          <strong className="text-primary">数据来源：</strong>
          订阅信息直接读取你在 <strong className="text-text-muted">订阅页</strong> 配置的账户模式（自动检测 + 手动切档位）。
          顾问只做分析，不再单独管理订阅，避免两处配置不一致。
        </div>
      </div>

      {/* Stats summary */}
      {result && (
        <div className="grid grid-cols-4 gap-3">
          <div className="card p-4">
            <div className="text-[11px] text-text-muted mb-1">订阅数</div>
            <div className="text-[22px] font-semibold">{result.subscription_count}</div>
          </div>
          <div className="card p-4">
            <div className="text-[11px] text-text-muted mb-1">月支出</div>
            <div className="text-[22px] font-semibold">¥{(result.total_monthly_usd * cnyRate).toFixed(0)}</div>
            <div className="text-[11px] text-text-faint">${result.total_monthly_usd.toFixed(0)}/月</div>
          </div>
          <div className="card p-4">
            <div className="text-[11px] text-text-muted mb-1">年支出</div>
            <div className="text-[22px] font-semibold">¥{(result.total_yearly_usd * cnyRate).toFixed(0)}</div>
            <div className="text-[11px] text-text-faint">${result.total_yearly_usd.toFixed(0)}/年</div>
          </div>
          <div className={cn("card p-4", result.total_savings_usd > 0 ? "border-success/30 bg-success/3" : "")}>
            <div className="text-[11px] text-text-muted mb-1">可节省</div>
            <div className={cn("text-[22px] font-semibold", result.total_savings_usd > 0 ? "text-success" : "text-text-faint")}>
              ¥{(result.total_savings_usd * cnyRate).toFixed(0)}
            </div>
            <div className="text-[11px] text-text-faint">
              {result.total_monthly_usd > 0 ? `月省 ${((result.total_savings_usd / result.total_monthly_usd) * 100).toFixed(0)}%` : "—"}
            </div>
          </div>
        </div>
      )}

      {/* Recommendations */}
      {result && result.recommendations.length > 0 && (
        <div>
          <div className="flex items-center gap-2 mb-3">
            <Sparkles size={14} className="text-primary" />
            <h2 className="text-[13px] font-medium text-text-muted">优化建议 ({result.recommendations.length})</h2>
          </div>
          <div className="space-y-3">
            {result.recommendations.map((r, i) => {
              const Icon = r.severity === "high" ? AlertTriangle : r.kind === "underused" ? TrendingDown : r.kind === "upgrade_recommended" ? TrendingUp : Info;
              return (
                <div key={i} className={cn("card p-4 border-l-4",
                  r.severity === "high" ? "border-l-danger" :
                  r.severity === "medium" ? "border-l-warning" : "border-l-primary")}>
                  <div className="flex items-start gap-3">
                    <Icon size={16} className={cn("mt-0.5 shrink-0",
                      r.severity === "high" ? "text-danger" :
                      r.severity === "medium" ? "text-warning" : "text-primary")} />
                    <div className="flex-1 min-w-0">
                      <div className="flex items-start justify-between gap-3 mb-1">
                        <h3 className="text-[14px] font-medium">{r.title}</h3>
                        {r.monthly_savings_usd > 0 && (
                          <span className={cn("text-[11px] px-2 py-0.5 rounded-full border shrink-0",
                            r.severity === "high" ? "bg-danger/10 text-danger border-danger/30" :
                            r.severity === "medium" ? "bg-warning/10 text-warning border-warning/30" :
                            "bg-primary/10 text-primary border-primary/30"
                          )}>月省 ¥{(r.monthly_savings_usd * cnyRate).toFixed(0)}</span>
                        )}
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

      {/* Subscription list with utilization */}
      {subs.length > 0 && (
        <div>
          <h2 className="text-[13px] font-medium text-text-muted mb-3">订阅使用情况 ({subs.length})</h2>
          <div className="card overflow-hidden">
            <table className="w-full text-[13px]">
              <thead>
                <tr className="text-text-faint text-[11px] border-b border-border-light bg-surface-lighter">
                  <th className="text-left px-4 py-2.5 font-medium">Provider</th>
                  <th className="text-left px-3 py-2.5 font-medium">类型</th>
                  <th className="text-right px-3 py-2.5 font-medium">月费</th>
                  <th className="text-right px-3 py-2.5 font-medium">30天请求</th>
                  <th className="text-right px-3 py-2.5 font-medium">API 等价（优化）</th>
                  <th className="text-center px-3 py-2.5 font-medium">利用率</th>
                </tr>
              </thead>
              <tbody>
                {subs.map(s => {
                  const cfg = utilizationConfig[s.utilization] || utilizationConfig.normal;
                  return (
                    <tr key={s.provider_id} className="border-b border-border-light/50 hover:bg-surface-lighter/50">
                      <td className="px-4 py-2.5">
                        <span className="inline-block w-2 h-2 rounded-full mr-2 align-middle" style={{ backgroundColor: pColor[s.provider_id] || "#666" }} />
                        <span className="font-medium">{s.provider_name}</span>
                      </td>
                      <td className="px-3 py-2.5 text-text-muted text-[12px]">{categoryLabel[s.category] || s.category}</td>
                      <td className="text-right px-3 py-2.5 font-medium">¥{(s.monthly_usd * cnyRate).toFixed(0)}</td>
                      <td className="text-right px-3 py-2.5 text-text-muted">{s.monthly_requests.toLocaleString()}</td>
                      <td className="text-right px-3 py-2.5 text-text-muted font-mono text-[12px]">¥{(s.reasonable_api_cost_usd * cnyRate).toFixed(0)}</td>
                      <td className="px-3 py-2.5 text-center">
                        <span className={cn("text-[11px] px-2 py-0.5 rounded-full font-medium", cfg.bg, cfg.color)}>
                          {cfg.label}
                        </span>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {subs.length === 0 && result && result.recommendations.length === 0 && (
        <div className="card p-12 text-center">
          <Calculator size={28} className="mx-auto mb-3 text-text-faint" />
          <p className="text-[14px] text-text-muted mb-1">尚未配置任何订阅</p>
          <p className="text-[12px] text-text-faint mb-4">去订阅页自动识别或手动切换你的订阅档位</p>
          <button onClick={() => navigate("/billing")}
            className="inline-flex items-center gap-2 px-4 py-2 bg-primary text-white rounded-[6px] text-[13px] font-medium hover:bg-primary-dark">
            <Zap size={13} /> 打开订阅页
          </button>
        </div>
      )}
    </div>
  );
}
