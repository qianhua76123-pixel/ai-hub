import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw, FolderGit2, Download, Pencil, Check } from "lucide-react";
import { AreaChart, Area, XAxis, YAxis, Tooltip, ResponsiveContainer, BarChart, Bar, Cell } from "recharts";
import { cn, formatTokens } from "../lib/utils";

interface ProviderUsage { provider_id: string; model: string; requests: number; input_tokens: number; output_tokens: number; cache_write_tokens: number; cache_read_tokens: number; cost: number; last_use: number; }
interface DailyUsage { day: string; tokens: number; }
interface TotalStats { requests: number; tokens: number; cost: number; }
interface HourlyUsage { hour: string; provider_id: string; requests: number; tokens: number; }
interface ProjectUsage { project: string; requests: number; tokens: number; cost: number; }

const pc: Record<string, string> = { anthropic: "#f59e0b", openai: "#0d9488", google: "#3b82f6", deepseek: "#6366f1", kimi: "#8b5cf6", qwen: "#f97316", zhipu: "#0ea5e9", groq: "#ef4444", mistral: "#f59e0b", cursor: "#0d9488" };
const pn: Record<string, string> = { anthropic: "Anthropic", openai: "OpenAI", google: "Google", deepseek: "DeepSeek", kimi: "Kimi", qwen: "通义千问", zhipu: "智谱", groq: "Groq", mistral: "Mistral", cursor: "Cursor" };
const tt = { backgroundColor: "var(--color-surface-light)", border: "1px solid var(--color-border)", borderRadius: "10px", color: "var(--color-text)", fontSize: 12, boxShadow: "0 4px 20px rgba(0,0,0,0.1)" };

export default function Usage() {
  const [byProvider, setByProvider] = useState<ProviderUsage[]>([]);
  const [dailyUsage, setDailyUsage] = useState<DailyUsage[]>([]);
  const [totalStats, setTotalStats] = useState<TotalStats>({ requests: 0, tokens: 0, cost: 0 });
  const [hourlyUsage, setHourlyUsage] = useState<HourlyUsage[]>([]);
  const [projectUsage, setProjectUsage] = useState<ProjectUsage[]>([]);
  const [viewMode, setViewMode] = useState<"model" | "project">("model");
  const [cnyRate, setCnyRate] = useState(7.2);
  const [editingProject, setEditingProject] = useState<string | null>(null);
  const [editValue, setEditValue] = useState("");
  const [exporting, setExporting] = useState(false);

  function load() {
    invoke<ProviderUsage[]>("get_usage_by_provider").then(setByProvider);
    invoke<DailyUsage[]>("get_daily_usage", { days: 30 }).then(setDailyUsage);
    invoke<TotalStats>("get_total_stats").then(setTotalStats);
    invoke<HourlyUsage[]>("get_hourly_usage", { hours: 48 }).then(setHourlyUsage);
    invoke<ProjectUsage[]>("get_usage_by_project").then(setProjectUsage);
    invoke<{ currency_rate: number }>("get_app_info").then(info => setCnyRate(info.currency_rate || 7.2)).catch(() => {});
  }
  useEffect(() => { load(); const i = setInterval(load, 15000); return () => clearInterval(i); }, []);

  async function renameProject(oldName: string) {
    if (!editValue.trim() || editValue === oldName) { setEditingProject(null); return; }
    try {
      await invoke("tag_traffic_project", { oldProject: oldName, newProject: editValue.trim() });
      load();
    } catch (e) { console.error(e); }
    setEditingProject(null);
  }

  async function exportCSV() {
    setExporting(true);
    try {
      const csv = await invoke<string>("export_usage_csv");
      const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a"); a.href = url;
      a.download = `ai-hub-usage-${new Date().toISOString().slice(0, 10)}.csv`;
      a.click(); URL.revokeObjectURL(url);
    } catch (e) { console.error(e); }
    setExporting(false);
  }
  async function exportJSON() {
    setExporting(true);
    try {
      const json = await invoke<string>("export_usage_json");
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a"); a.href = url;
      a.download = `ai-hub-usage-${new Date().toISOString().slice(0, 10)}.json`;
      a.click(); URL.revokeObjectURL(url);
    } catch (e) { console.error(e); }
    setExporting(false);
  }

  const ps: Record<string, { cost: number }> = {};
  byProvider.forEach((p) => { if (!ps[p.provider_id]) ps[p.provider_id] = { cost: 0 }; ps[p.provider_id].cost += p.cost; });
  const cd = Object.entries(ps).map(([id, d]) => ({ name: pn[id] || id, cost: Number(d.cost.toFixed(2)), color: pc[id] || "#666" })).sort((a, b) => b.cost - a.cost);

  const ha: Record<string, { h: string; r: number }> = {};
  hourlyUsage.forEach((x) => { if (!ha[x.hour]) ha[x.hour] = { h: x.hour.slice(11, 16), r: 0 }; ha[x.hour].r += x.requests; });
  const hc = Object.values(ha).slice(-24);

  function timeAgo(ts: number) { const d = Math.floor((Date.now() - ts) / 1000); if (d < 60) return "刚刚"; if (d < 3600) return Math.floor(d / 60) + "m"; if (d < 86400) return Math.floor(d / 3600) + "h"; return Math.floor(d / 86400) + "d"; }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div><h1 className="text-[22px] font-semibold tracking-tight">用量统计</h1><p className="text-[13px] text-text-muted mt-0.5">Token 消耗与费用趋势</p></div>
        <div className="flex items-center gap-2">
          <button onClick={exportCSV} disabled={exporting}
            className="flex items-center gap-1.5 text-[12px] text-text-muted hover:text-primary px-2.5 py-1.5 rounded-[6px] hover:bg-primary/5 transition-colors">
            <Download size={12} /> CSV
          </button>
          <button onClick={exportJSON} disabled={exporting}
            className="flex items-center gap-1.5 text-[12px] text-text-muted hover:text-primary px-2.5 py-1.5 rounded-[6px] hover:bg-primary/5 transition-colors">
            <Download size={12} /> JSON
          </button>
          <button onClick={load} className="flex items-center gap-1.5 text-[13px] text-primary hover:text-primary-dark transition-colors"><RefreshCw size={13} /> 刷新</button>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-4">
        {[{ l: "累计请求", v: totalStats.requests.toLocaleString() + " 次", c: "#0d9488" }, { l: "累计 Token", v: formatTokens(totalStats.tokens), c: "#f59e0b" }, { l: "累计费用", v: "¥" + (totalStats.cost * cnyRate).toFixed(2), c: "#ef4444" }].map((s) => (
          <div key={s.l} className="card p-5">
            <div className="text-[12px] text-text-muted mb-1.5">{s.l}</div>
            <div className="stat-value text-[24px]" style={{ color: s.c }}>{s.v}</div>
          </div>
        ))}
      </div>

      <div className="grid grid-cols-5 gap-4">
        <div className="col-span-3 card p-5">
          <h2 className="text-[13px] font-medium text-text-muted mb-4">近 30 天</h2>
          {dailyUsage.length > 0 ? (
            <ResponsiveContainer width="100%" height={180}>
              <AreaChart data={dailyUsage}>
                <defs><linearGradient id="ug" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stopColor="#0d9488" stopOpacity={0.1} /><stop offset="100%" stopColor="#0d9488" stopOpacity={0} /></linearGradient></defs>
                <XAxis dataKey="day" stroke="#e2e5eb" fontSize={10} tickLine={false} axisLine={false} tick={{ fill: "#9ca3af" }} tickFormatter={(v) => v.slice(5)} />
                <YAxis stroke="#e2e5eb" fontSize={10} tickLine={false} axisLine={false} tick={{ fill: "#9ca3af" }} tickFormatter={formatTokens} />
                <Tooltip contentStyle={tt} formatter={(v) => [formatTokens(Number(v)), "Token"]} />
                <Area type="monotone" dataKey="tokens" stroke="#0d9488" fill="url(#ug)" strokeWidth={2} dot={false} />
              </AreaChart>
            </ResponsiveContainer>
          ) : <div className="h-44 flex items-center justify-center text-text-faint text-[13px]">暂无数据</div>}
        </div>
        <div className="col-span-2 card p-5">
          <h2 className="text-[13px] font-medium text-text-muted mb-4">费用分布</h2>
          {cd.length > 0 ? (
            <ResponsiveContainer width="100%" height={180}>
              <BarChart data={cd} layout="vertical">
                <XAxis type="number" stroke="#e2e5eb" fontSize={10} tickLine={false} axisLine={false} tick={{ fill: "#9ca3af" }} tickFormatter={(v) => `$${v}`} />
                <YAxis type="category" dataKey="name" stroke="#e2e5eb" fontSize={11} tickLine={false} axisLine={false} tick={{ fill: "#6b7080" }} width={65} />
                <Tooltip contentStyle={tt} formatter={(v) => [`$${Number(v).toFixed(2)}`, ""]} />
                <Bar dataKey="cost" radius={[0, 4, 4, 0]}>{cd.map((d, i) => <Cell key={i} fill={d.color} />)}</Bar>
              </BarChart>
            </ResponsiveContainer>
          ) : <div className="h-44 flex items-center justify-center text-text-faint text-[13px]">暂无数据</div>}
        </div>
      </div>

      {hc.length > 0 && <div className="card p-5"><h2 className="text-[13px] font-medium text-text-muted mb-3">24h 活跃度</h2>
        <ResponsiveContainer width="100%" height={60}><BarChart data={hc}><XAxis dataKey="h" stroke="#e2e5eb" fontSize={9} tickLine={false} axisLine={false} tick={{ fill: "#9ca3af" }} /><Tooltip contentStyle={tt} formatter={(v) => [v, "请求"]} /><Bar dataKey="r" fill="#0d9488" radius={[3, 3, 0, 0]} opacity={0.5} /></BarChart></ResponsiveContainer>
      </div>}

      {/* View mode toggle */}
      <div className="flex items-center gap-2">
        <div className="flex gap-1 bg-surface-light rounded-[8px] p-0.5 border border-border">
          <button onClick={() => setViewMode("model")}
            className={cn("px-3 py-1.5 rounded-[6px] text-[12px] transition-all",
              viewMode === "model" ? "bg-surface-lighter text-text font-medium shadow-sm" : "text-text-muted hover:text-text")}>
            按模型
          </button>
          <button onClick={() => setViewMode("project")}
            className={cn("flex items-center gap-1.5 px-3 py-1.5 rounded-[6px] text-[12px] transition-all",
              viewMode === "project" ? "bg-surface-lighter text-text font-medium shadow-sm" : "text-text-muted hover:text-text")}>
            <FolderGit2 size={12} />
            按项目
          </button>
        </div>
      </div>

      {/* Project usage table */}
      {viewMode === "project" && projectUsage.length > 0 && (
        <div className="space-y-4">
          <div className="card p-5">
            <h2 className="text-[13px] font-medium text-text-muted mb-4">项目费用分布</h2>
            <ResponsiveContainer width="100%" height={Math.max(120, projectUsage.length * 40)}>
              <BarChart data={projectUsage.slice(0, 10)} layout="vertical">
                <XAxis type="number" stroke="#e2e5eb" fontSize={10} tickLine={false} axisLine={false} tick={{ fill: "#9ca3af" }} tickFormatter={(v) => `$${v}`} />
                <YAxis type="category" dataKey="project" stroke="#e2e5eb" fontSize={11} tickLine={false} axisLine={false} tick={{ fill: "#6b7080" }} width={100} />
                <Tooltip contentStyle={tt} formatter={(v) => [`$${Number(v).toFixed(4)}`, "费用"]} />
                <Bar dataKey="cost" fill="#0d9488" radius={[0, 4, 4, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
          <div className="card overflow-hidden">
            <table className="w-full text-[13px]"><thead><tr className="text-text-faint text-[11px] border-b border-border-light bg-surface-lighter">
              <th className="text-left px-4 py-2.5 font-medium">项目</th><th className="text-right px-4 py-2.5 font-medium">请求</th><th className="text-right px-4 py-2.5 font-medium">Token</th><th className="text-right px-4 py-2.5 font-medium">费用</th>
            </tr></thead><tbody>{projectUsage.map((p) => (
              <tr key={p.project} className="border-b border-border-light/50 hover:bg-surface-lighter/50 transition-colors">
                <td className="px-4 py-2.5 font-medium">
                  {editingProject === p.project ? (
                    <div className="flex items-center gap-1.5">
                      <input value={editValue} onChange={(e) => setEditValue(e.target.value)}
                        onKeyDown={(e) => e.key === "Enter" && renameProject(p.project)}
                        autoFocus
                        className="bg-surface border border-primary rounded px-2 py-0.5 text-[12px] w-32 outline-none" />
                      <button onClick={() => renameProject(p.project)} className="text-primary hover:text-primary-dark"><Check size={13} /></button>
                    </div>
                  ) : (
                    <span className="group cursor-default">
                      <FolderGit2 size={13} className="inline mr-2 text-text-faint" />{p.project}
                      <button onClick={() => { setEditingProject(p.project); setEditValue(p.project); }}
                        className="ml-2 opacity-0 group-hover:opacity-100 text-text-faint hover:text-primary transition-opacity">
                        <Pencil size={11} />
                      </button>
                    </span>
                  )}
                </td>
                <td className="text-right px-4 py-2.5 text-text-muted">{p.requests.toLocaleString()}</td>
                <td className="text-right px-4 py-2.5 text-text-muted font-mono text-[12px]">{formatTokens(p.tokens)}</td>
                <td className="text-right px-4 py-2.5 font-medium">¥{(p.cost * cnyRate).toFixed(2)}</td>
              </tr>))}</tbody></table>
          </div>
        </div>
      )}

      {viewMode === "project" && projectUsage.length === 0 && (
        <div className="card p-12 text-center">
          <FolderGit2 size={28} className="mx-auto mb-3 text-text-faint" />
          <p className="text-[13px] text-text-muted">暂无项目归因数据</p>
          <p className="text-[12px] text-text-faint mt-1">通过 AI Hub 代理发送请求后自动归因</p>
        </div>
      )}

      {/* Model usage table */}
      {viewMode === "model" && byProvider.length > 0 && <div className="card overflow-hidden">
        <table className="w-full text-[13px]"><thead><tr className="text-text-faint text-[11px] border-b border-border-light bg-surface-lighter">
          <th className="text-left px-4 py-2.5 font-medium">模型</th>
          <th className="text-right px-3 py-2.5 font-medium">请求</th>
          <th className="text-right px-3 py-2.5 font-medium">新输入</th>
          <th className="text-right px-3 py-2.5 font-medium"><span className="text-warning">缓存写</span></th>
          <th className="text-right px-3 py-2.5 font-medium"><span className="text-success">缓存读</span></th>
          <th className="text-right px-3 py-2.5 font-medium">输出</th>
          <th className="text-right px-3 py-2.5 font-medium">费用</th>
          <th className="text-right px-3 py-2.5 font-medium">最后</th>
        </tr></thead><tbody>{byProvider.map((p) => (
          <tr key={`${p.provider_id}-${p.model}`} className="border-b border-border-light/50 hover:bg-surface-lighter/50 transition-colors">
            <td className="px-4 py-2.5"><span className="inline-block w-2 h-2 rounded-full mr-2 align-middle" style={{ backgroundColor: pc[p.provider_id] }} />{p.model}<span className="text-text-faint ml-2">{pn[p.provider_id]}</span></td>
            <td className="text-right px-3 py-2.5 text-text-muted">{p.requests.toLocaleString()}</td>
            <td className="text-right px-3 py-2.5 text-text-muted font-mono text-[12px]">{formatTokens(p.input_tokens)}</td>
            <td className="text-right px-3 py-2.5 font-mono text-[12px] text-warning">{p.cache_write_tokens > 0 ? formatTokens(p.cache_write_tokens) : "-"}</td>
            <td className="text-right px-3 py-2.5 font-mono text-[12px] text-success">{p.cache_read_tokens > 0 ? formatTokens(p.cache_read_tokens) : "-"}</td>
            <td className="text-right px-3 py-2.5 text-text-muted font-mono text-[12px]">{formatTokens(p.output_tokens)}</td>
            <td className="text-right px-3 py-2.5 font-medium">¥{(p.cost * cnyRate).toFixed(2)}</td>
            <td className="text-right px-3 py-2.5 text-text-faint text-[12px]">{timeAgo(p.last_use)}</td>
          </tr>))}</tbody></table>
      </div>}
    </div>
  );
}
