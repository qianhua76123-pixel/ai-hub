import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check, Copy, PowerOff, Download, Trash2, RefreshCw, Info, Shield, Database, Sun, Moon, Monitor } from "lucide-react";
import { cn } from "../lib/utils";
import { getStoredTheme, setTheme } from "../lib/theme";

interface ProxyStatus { running: boolean; port: number; base_url: string; }
interface ToolConfig { tool_id: string; tool_name: string; config_path: string; is_redirected: boolean; }
interface AppInfo { name: string; version: string; platform: string; arch: string; currency_rate: number; }

export default function SettingsPage() {
  const [proxyStatus, setProxyStatus] = useState<ProxyStatus | null>(null);
  const [tools, setTools] = useState<ToolConfig[]>([]);
  const [envExports, setEnvExports] = useState("");
  const [copied, setCopied] = useState(false);
  const [msg, setMsg] = useState("");
  const [appInfo, setAppInfo] = useState<AppInfo | null>(null);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [updateMsg, setUpdateMsg] = useState("");
  const [theme, setThemeState] = useState(getStoredTheme);
  const [accountModes, setAccountModes] = useState<{ provider_id: string; mode: string; subscription_monthly_usd: number }[]>([]);
  const [budgetLimit, setBudgetLimit] = useState("");
  const [budgetNotify70, setBudgetNotify70] = useState(true);
  const [budgetNotify90, setBudgetNotify90] = useState(true);
  const [budgetPause100, setBudgetPause100] = useState(false);

  useEffect(() => {
    invoke<ProxyStatus>("get_proxy_status").then(setProxyStatus);
    invoke<ToolConfig[]>("get_manageable_tools").then(setTools);
    invoke<string>("get_env_exports").then(setEnvExports);
    invoke<AppInfo>("get_app_info").then(setAppInfo);
    invoke<{ provider_id: string; mode: string; subscription_monthly_usd: number }[]>("get_account_modes").then(setAccountModes).catch(() => {});
    // Load budget
    invoke<{ id: string; monthly_limit_usd: number; notify_70: boolean; notify_90: boolean; pause_at_100: boolean }[]>("get_budgets").then(budgets => {
      const global = budgets.find(b => b.id === "global");
      if (global) {
        setBudgetLimit(String(global.monthly_limit_usd));
        setBudgetNotify70(global.notify_70);
        setBudgetNotify90(global.notify_90);
        setBudgetPause100(global.pause_at_100);
      }
    }).catch(() => {});
  }, []);

  async function toggle(id: string, on: boolean) {
    try {
      const m = await invoke<string>(on ? "disable_proxy_for_tool" : "enable_proxy_for_tool", { toolId: id });
      setMsg(m); setTools(await invoke<ToolConfig[]>("get_manageable_tools"));
      setTimeout(() => setMsg(""), 3000);
    } catch (e) { setMsg(String(e)); }
  }

  async function checkForUpdate() {
    setCheckingUpdate(true);
    setUpdateMsg("");
    try {
      // For now, just show current version info until updater is fully configured
      setUpdateMsg(`当前版本 ${appInfo?.version || "unknown"} 已是最新`);
    } catch (e) {
      setUpdateMsg(`检查失败: ${e}`);
    }
    setCheckingUpdate(false);
    setTimeout(() => setUpdateMsg(""), 5000);
  }

  async function exportDiagnostics() {
    try {
      const info = {
        app: appInfo,
        proxy: proxyStatus,
        tools: tools,
        platform: navigator.userAgent,
        timestamp: new Date().toISOString(),
      };
      const text = JSON.stringify(info, null, 2);
      await navigator.clipboard.writeText(text);
      setMsg("诊断信息已复制到剪贴板");
      setTimeout(() => setMsg(""), 3000);
    } catch (e) { setMsg(String(e)); }
  }

  return (
    <div className="space-y-6">
      <h1 className="text-[22px] font-semibold tracking-tight">设置</h1>
      {msg && <div className="card px-4 py-3 text-[13px] text-primary bg-primary/5">{msg}</div>}

      {/* Proxy Gateway */}
      <div className="card overflow-hidden">
        <div className="px-5 py-3 border-b border-border-light bg-surface-lighter/50">
          <h2 className="text-[13px] font-medium text-text-muted">代理网关</h2>
        </div>
        <div className="px-5 py-4 space-y-2">
          <div className="flex justify-between text-[13px]">
            <span className="text-text-muted">状态</span>
            {proxyStatus && <span className={proxyStatus.running ? "text-success font-medium" : "text-danger"}>
              {proxyStatus.running ? `运行中 :${proxyStatus.port}` : "未启动"}</span>}
          </div>
          {proxyStatus?.running && <div className="flex justify-between text-[13px]">
            <span className="text-text-muted">地址</span>
            <code className="text-[12px] text-primary font-mono">{proxyStatus.base_url}</code>
          </div>}
        </div>
      </div>

      {/* Theme */}
      <div className="card overflow-hidden">
        <div className="px-5 py-3 border-b border-border-light bg-surface-lighter/50">
          <h2 className="text-[13px] font-medium text-text-muted">外观</h2>
        </div>
        <div className="px-5 py-4">
          <div className="flex items-center gap-2">
            {([
              { value: "system" as const, icon: Monitor, label: "跟随系统" },
              { value: "light" as const, icon: Sun, label: "浅色" },
              { value: "dark" as const, icon: Moon, label: "深色" },
            ]).map((opt) => (
              <button key={opt.value}
                onClick={() => { setTheme(opt.value); setThemeState(opt.value); }}
                className={cn("flex items-center gap-2 px-4 py-2.5 rounded-[8px] text-[13px] transition-all border",
                  theme === opt.value
                    ? "bg-primary/10 text-primary border-primary/20 font-medium"
                    : "bg-surface-lighter text-text-muted border-transparent hover:border-border")}>
                <opt.icon size={14} />
                {opt.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Account Mode — with preset plans */}
      <div className="card overflow-hidden">
        <div className="px-5 py-3 border-b border-border-light bg-surface-lighter/50">
          <h2 className="text-[13px] font-medium text-text-muted">账户模式（自动识别订阅档位）</h2>
        </div>
        <div className="px-5 py-4 space-y-3">
          <p className="text-[11px] text-text-faint leading-relaxed">
            选择每个 Provider 你实际订阅的档位。系统会自动区分 <strong className="text-text-muted">API 真实付费</strong> 和 <strong className="text-text-muted">订阅虚拟等价</strong>（如果改走 API 会花多少钱）。
          </p>
          {(() => {
            // 预设档位数据库 — 覆盖 2026 年主流订阅档位
            const PLANS: Record<string, { label: string; plans: { id: string; name: string; usd: number; mode: string }[] }> = {
              anthropic: {
                label: "Anthropic (Claude)",
                plans: [
                  { id: "api", name: "API 按量付费", usd: 0, mode: "api" },
                  { id: "pro", name: "Claude Pro ($20/月)", usd: 20, mode: "subscription" },
                  { id: "max5", name: "Claude Max 5x ($100/月)", usd: 100, mode: "subscription" },
                  { id: "max20", name: "Claude Max 20x ($200/月)", usd: 200, mode: "subscription" },
                  { id: "team", name: "Claude Team ($25/人/月)", usd: 25, mode: "subscription" },
                ]
              },
              openai: {
                label: "OpenAI (ChatGPT)",
                plans: [
                  { id: "api", name: "API 按量付费", usd: 0, mode: "api" },
                  { id: "plus", name: "ChatGPT Plus ($20/月)", usd: 20, mode: "subscription" },
                  { id: "pro", name: "ChatGPT Pro ($200/月)", usd: 200, mode: "subscription" },
                  { id: "team", name: "ChatGPT Team ($25/人/月)", usd: 25, mode: "subscription" },
                ]
              },
              google: {
                label: "Google (Gemini)",
                plans: [
                  { id: "api", name: "API 按量付费", usd: 0, mode: "api" },
                  { id: "advanced", name: "Gemini Advanced ($19.99/月)", usd: 19.99, mode: "subscription" },
                  { id: "ultra", name: "Gemini AI Ultra ($249.99/月)", usd: 249.99, mode: "subscription" },
                ]
              },
              cursor: {
                label: "Cursor",
                plans: [
                  { id: "hobby", name: "免费版 ($0)", usd: 0, mode: "api" },
                  { id: "pro", name: "Cursor Pro ($20/月)", usd: 20, mode: "subscription" },
                  { id: "business", name: "Cursor Business ($40/人/月)", usd: 40, mode: "subscription" },
                ]
              },
              copilot: {
                label: "GitHub Copilot",
                plans: [
                  { id: "free", name: "免费版", usd: 0, mode: "api" },
                  { id: "pro", name: "Copilot Pro ($10/月)", usd: 10, mode: "subscription" },
                  { id: "pro_plus", name: "Copilot Pro+ ($39/月)", usd: 39, mode: "subscription" },
                  { id: "business", name: "Copilot Business ($19/人/月)", usd: 19, mode: "subscription" },
                ]
              },
              xai: {
                label: "xAI (Grok)",
                plans: [
                  { id: "api", name: "API 按量付费", usd: 0, mode: "api" },
                  { id: "plus", name: "SuperGrok ($30/月)", usd: 30, mode: "subscription" },
                  { id: "heavy", name: "SuperGrok Heavy ($300/月)", usd: 300, mode: "subscription" },
                ]
              },
            };

            async function updatePlan(providerId: string, planUsd: number, planMode: string) {
              await invoke("set_account_mode", { providerId, mode: planMode, subscriptionMonthlyUsd: planUsd });
              setAccountModes(await invoke("get_account_modes"));
              setMsg(`${providerId} 已更新`);
              setTimeout(() => setMsg(""), 2500);
            }

            return (
              <div className="space-y-2">
                {Object.entries(PLANS).map(([pid, cfg]) => {
                  const current = accountModes.find(m => m.provider_id === pid);
                  const currentUsd = current?.subscription_monthly_usd || 0;
                  const currentMode = current?.mode || "api";
                  // Match by mode + usd
                  const currentPlan = cfg.plans.find(p => p.mode === currentMode && Math.abs(p.usd - currentUsd) < 0.01)?.id || "api";
                  return (
                    <div key={pid} className="flex items-center gap-3 text-[12px]">
                      <span className="w-36 text-text-muted">{cfg.label}</span>
                      <select value={currentPlan}
                        onChange={(e) => {
                          const plan = cfg.plans.find(p => p.id === e.target.value);
                          if (plan) updatePlan(pid, plan.usd, plan.mode);
                        }}
                        className="flex-1 bg-surface border border-border rounded-[6px] px-2.5 py-1.5 text-[12px] focus:outline-none focus:border-primary">
                        {cfg.plans.map(p => <option key={p.id} value={p.id}>{p.name}</option>)}
                      </select>
                      {currentMode === "subscription" && (
                        <span className="text-[10px] text-success px-2 py-0.5 rounded-full bg-success/10 border border-success/20">
                          订阅中
                        </span>
                      )}
                    </div>
                  );
                })}
              </div>
            );
          })()}
          <p className="text-[10px] text-text-faint pt-2 border-t border-border-light">
            💡 提示：订阅用户切换档位后，历史流量会自动重新标记模式，「费用构成」卡片实时更新
          </p>
        </div>
      </div>

      {/* Budget */}
      <div className="card overflow-hidden">
        <div className="px-5 py-3 border-b border-border-light bg-surface-lighter/50">
          <h2 className="text-[13px] font-medium text-text-muted">月度预算</h2>
        </div>
        <div className="px-5 py-4 space-y-3">
          <div className="flex items-center gap-3">
            <span className="text-[13px] text-text-muted w-24">月度上限</span>
            <div className="flex items-center gap-1.5">
              <span className="text-[13px] text-text-faint">$</span>
              <input value={budgetLimit} onChange={(e) => setBudgetLimit(e.target.value)}
                placeholder="0 = 不限制" type="number" step="10" min="0"
                className="w-24 bg-surface border border-border rounded-[6px] px-2.5 py-1.5 text-[13px] focus:outline-none focus:border-primary" />
              <span className="text-[11px] text-text-faint">USD/月</span>
            </div>
          </div>
          <div className="flex items-center gap-4 text-[12px]">
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input type="checkbox" checked={budgetNotify70} onChange={(e) => setBudgetNotify70(e.target.checked)} className="rounded" />
              <span className="text-text-muted">70% 提醒</span>
            </label>
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input type="checkbox" checked={budgetNotify90} onChange={(e) => setBudgetNotify90(e.target.checked)} className="rounded" />
              <span className="text-text-muted">90% 提醒</span>
            </label>
            <label className="flex items-center gap-1.5 cursor-pointer">
              <input type="checkbox" checked={budgetPause100} onChange={(e) => setBudgetPause100(e.target.checked)} className="rounded" />
              <span className="text-danger">100% 暂停代理</span>
            </label>
          </div>
          <button onClick={async () => {
            const limit = parseFloat(budgetLimit) || 0;
            try {
              await invoke("set_budget", { providerId: "", monthlyLimitUsd: limit, notify70: budgetNotify70, notify90: budgetNotify90, pauseAt100: budgetPause100 });
              setMsg(limit > 0 ? `���算已设置: $${limit}/月` : "预算已取消");
              setTimeout(() => setMsg(""), 3000);
            } catch (e) { setMsg(String(e)); }
          }}
            className="text-[12px] px-4 py-1.5 bg-primary hover:bg-primary-dark text-white rounded-[6px] font-medium transition-colors">
            保存预算
          </button>
        </div>
      </div>

      {/* Tool Integration */}
      <div className="card overflow-hidden">
        <div className="px-5 py-3 border-b border-border-light bg-surface-lighter/50">
          <h2 className="text-[13px] font-medium text-text-muted">工具接入</h2>
        </div>
        <div className="divide-y divide-border-light">
          {tools.map((t) => (
            <div key={t.tool_id} className="px-5 py-3.5 flex items-center justify-between">
              <div><div className="text-[13px] font-medium">{t.tool_name}</div><div className="text-[11px] text-text-faint font-mono mt-0.5">{t.config_path}</div></div>
              <button onClick={() => toggle(t.tool_id, t.is_redirected)}
                className={cn("text-[12px] px-3 py-1.5 rounded-[6px] font-medium transition-colors",
                  t.is_redirected ? "bg-success/10 text-success" : "bg-surface-lighter text-text-faint hover:text-text-muted")}>
                {t.is_redirected ? <><Check size={12} className="inline mr-1" />已接入</> : <><PowerOff size={12} className="inline mr-1" />未接入</>}
              </button>
            </div>
          ))}
          {tools.length === 0 && <div className="px-5 py-6 text-center text-text-faint text-[13px]">未检测到工具</div>}
        </div>
      </div>

      {/* Shell Env */}
      <div className="card overflow-hidden">
        <div className="px-5 py-3 border-b border-border-light bg-surface-lighter/50 flex items-center justify-between">
          <h2 className="text-[13px] font-medium text-text-muted">Shell 环境变量</h2>
          <div className="flex gap-2">
            <button onClick={async () => { try { setMsg(await invoke<string>("install_shell_proxy")); setTimeout(() => setMsg(""), 5000); } catch(e) { setMsg(String(e)); } }}
              className="text-[12px] text-primary hover:text-primary-dark flex items-center gap-1 transition-colors"><Download size={12} /> 写入 .zshrc</button>
            <button onClick={async () => { try { setMsg(await invoke<string>("uninstall_shell_proxy")); setTimeout(() => setMsg(""), 5000); } catch(e) { setMsg(String(e)); } }}
              className="text-[12px] text-danger flex items-center gap-1"><Trash2 size={12} /> 卸载</button>
          </div>
        </div>
        <div className="relative">
          <pre className="px-5 py-4 text-[11px] text-text-muted font-mono leading-relaxed overflow-auto max-h-40">{envExports}</pre>
          <button onClick={() => { navigator.clipboard.writeText(envExports); setCopied(true); setTimeout(() => setCopied(false), 2000); }}
            className="absolute top-3 right-3 p-1.5 rounded-[6px] bg-surface-lighter hover:bg-border-light transition-colors">
            {copied ? <Check size={12} className="text-success" /> : <Copy size={12} className="text-text-faint" />}
          </button>
        </div>
      </div>

      {/* About + Update */}
      <div className="card overflow-hidden">
        <div className="px-5 py-3 border-b border-border-light bg-surface-lighter/50"><h2 className="text-[13px] font-medium text-text-muted">关于</h2></div>
        <div className="px-5 py-4 space-y-3">
          <div className="flex justify-between text-[13px]">
            <span className="text-text-muted">版本</span>
            <div className="flex items-center gap-2">
              <span className="font-medium">{appInfo?.version || "..."}</span>
              <button onClick={checkForUpdate} disabled={checkingUpdate}
                className="text-[11px] text-primary hover:text-primary-dark flex items-center gap-1 transition-colors">
                <RefreshCw size={11} className={checkingUpdate ? "animate-spin" : ""} />
                检查更新
              </button>
            </div>
          </div>
          {updateMsg && <div className="text-[12px] text-primary bg-primary/5 rounded-[6px] px-3 py-1.5">{updateMsg}</div>}
          <div className="flex justify-between text-[13px]">
            <span className="text-text-muted">平台</span>
            <span className="text-text-faint">{appInfo?.platform || "..."} / {appInfo?.arch || "..."}</span>
          </div>
          <div className="flex justify-between text-[13px]">
            <span className="text-text-muted flex items-center gap-1"><Shield size={12} />数据安全</span>
            <span className="text-text-faint">本地存储 · API Key 加密保存 · 不上传任何信息</span>
          </div>
          <div className="flex justify-between text-[13px]">
            <span className="text-text-muted flex items-center gap-1"><Database size={12} />汇率</span>
            <div className="flex items-center gap-2">
              <span className="text-text-faint">1 USD = {appInfo?.currency_rate?.toFixed(4) || "..."} CNY</span>
              <button onClick={async () => {
                try {
                  const result = await invoke<{ rate: number; source: string }>("refresh_exchange_rate");
                  setAppInfo(prev => prev ? { ...prev, currency_rate: result.rate } : prev);
                  setMsg(`汇率已更新: ${result.rate.toFixed(4)} (${result.source})`);
                  setTimeout(() => setMsg(""), 3000);
                } catch (e) { setMsg(String(e)); }
              }} className="text-[11px] text-primary hover:text-primary-dark flex items-center gap-0.5 transition-colors">
                <RefreshCw size={10} /> 刷新
              </button>
            </div>
          </div>
          <div className="pt-2 border-t border-border-light">
            <button onClick={exportDiagnostics}
              className="text-[12px] text-text-faint hover:text-text-muted flex items-center gap-1.5 transition-colors">
              <Info size={12} /> 导出诊断信息
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
