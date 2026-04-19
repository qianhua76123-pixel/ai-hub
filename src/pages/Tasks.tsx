import { useEffect, useState, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Plus, Send, Loader2, CheckCircle2, XCircle, Clock, ChevronDown, ChevronUp, DollarSign, Timer, Cpu, Sparkles, Code, Brain, PenLine, MessageCircle, ShieldCheck } from "lucide-react";
import { cn, formatNumber } from "../lib/utils";

interface TaskRecord {
  id: string; title: string; prompt: string; task_type: string; provider_id: string; model: string;
  status: string; result: string; input_tokens: number; output_tokens: number; estimated_cost: number;
  latency_ms: number; error_msg: string; parent_id: string | null; created_at: number; started_at: number | null; completed_at: number | null;
}
interface ProviderEndpoint { id: string; name: string; default_model: string; }
interface RouteRecommendation { provider_id: string; model_id: string; model_name: string; reason: string; score: number; cost_per_m_input: number; arena_score: number; }
interface TaskClassification { task_type: string; confidence: number; recommendations: RouteRecommendation[]; }

const taskTypeConfig: Record<string, { icon: typeof Code; label: string; color: string; bg: string }> = {
  code:      { icon: Code,          label: "代码",   color: "text-primary",  bg: "bg-primary/10" },
  reasoning: { icon: Brain,         label: "推理",   color: "text-warning",  bg: "bg-warning/10" },
  writing:   { icon: PenLine,       label: "写作",   color: "text-success",  bg: "bg-success/10" },
  chat:      { icon: MessageCircle, label: "对话",   color: "text-text-muted", bg: "bg-surface-lighter" },
};

const statusCfg: Record<string, { icon: typeof Loader2; color: string; label: string; bg: string }> = {
  pending:   { icon: Clock,        color: "text-text-faint",  label: "等待中", bg: "bg-surface-lighter" },
  running:   { icon: Loader2,      color: "text-primary",     label: "执行中", bg: "bg-primary/8" },
  completed: { icon: CheckCircle2, color: "text-success",     label: "已完成", bg: "bg-success/8" },
  failed:    { icon: XCircle,      color: "text-danger",      label: "失败",   bg: "bg-danger/8" },
};

export default function Tasks() {
  const [tasks, setTasks] = useState<TaskRecord[]>([]);
  const [providers, setProviders] = useState<ProviderEndpoint[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [expandedTask, setExpandedTask] = useState<string | null>(null);
  const [subtasks, setSubtasks] = useState<Record<string, TaskRecord[]>>({});
  const [title, setTitle] = useState("");
  const [prompt, setPrompt] = useState("");
  const [selectedProvider, setSelectedProvider] = useState("all");
  const [creating, setCreating] = useState(false);
  const [routing, setRouting] = useState<TaskClassification | null>(null);
  const [routingLoading, setRoutingLoading] = useState(false);
  const [verifying, setVerifying] = useState<string | null>(null);
  const [routeDecisions, setRouteDecisions] = useState<Record<string, { prompt_type: string; confidence: number; recommended_model: string; recommended_provider: string; actual_model: string; actual_provider: string } | null>>({});
  const pollRef = useRef<ReturnType<typeof setInterval>>(undefined);
  const routeTimerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    invoke<ProviderEndpoint[]>("get_available_providers").then(setProviders);
    loadTasks();
    pollRef.current = setInterval(loadTasks, 3000);
    return () => clearInterval(pollRef.current);
  }, []);

  // Debounced smart routing
  useEffect(() => {
    if (!prompt.trim() || prompt.trim().length < 5) { setRouting(null); return; }
    clearTimeout(routeTimerRef.current);
    routeTimerRef.current = setTimeout(async () => {
      setRoutingLoading(true);
      try {
        const result = await invoke<TaskClassification>("recommend_route", { prompt });
        setRouting(result);
      } catch { setRouting(null); }
      setRoutingLoading(false);
    }, 500);
    return () => clearTimeout(routeTimerRef.current);
  }, [prompt]);

  function loadTasks() { invoke<TaskRecord[]>("get_tasks", { limit: 50 }).then(setTasks); }

  async function toggleExpand(id: string) {
    if (expandedTask === id) { setExpandedTask(null); return; }
    setExpandedTask(id);
    const subs = await invoke<TaskRecord[]>("get_subtasks", { parentId: id });
    setSubtasks((p) => ({ ...p, [id]: subs }));
    // Fetch route decision
    if (!routeDecisions[id]) {
      invoke<{ prompt_type: string; confidence: number; recommended_model: string; recommended_provider: string; actual_model: string; actual_provider: string } | null>("get_route_decision", { taskId: id })
        .then(rd => setRouteDecisions(p => ({ ...p, [id]: rd ?? null })))
        .catch(() => {});
    }
  }

  useEffect(() => {
    if (!expandedTask) return;
    const i = setInterval(async () => {
      const subs = await invoke<TaskRecord[]>("get_subtasks", { parentId: expandedTask });
      setSubtasks((p) => ({ ...p, [expandedTask!]: subs }));
    }, 2000);
    return () => clearInterval(i);
  }, [expandedTask]);

  async function handleCreate() {
    if (!prompt.trim()) return;
    setCreating(true);
    const t = title.trim() || prompt.slice(0, 40) + (prompt.length > 40 ? "..." : "");
    try {
      if (selectedProvider === "all") await invoke("create_multi_agent_task", { title: t, prompt });
      else { const prov = providers.find((p) => p.id === selectedProvider); await invoke("create_task", { title: t, prompt, providerId: selectedProvider, model: prov?.default_model || "" }); }
      setTitle(""); setPrompt(""); setShowCreate(false); loadTasks();
    } catch (e) { console.error(e); }
    setCreating(false);
  }

  async function handleVerify(taskId: string) {
    setVerifying(taskId);
    try {
      await invoke("verify_task", { taskId });
      loadTasks();
    } catch (e) { console.error(e); }
    setVerifying(null);
  }

  const parentTasks = tasks.filter((t) => !t.parent_id);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-[22px] font-semibold tracking-tight">任务中心</h1>
          <p className="text-[13px] text-text-muted mt-0.5">向 AI Agent 发送任务，实时追踪执行状态</p>
        </div>
        <button onClick={() => setShowCreate(!showCreate)}
          className={cn("flex items-center gap-2 px-4 py-2 rounded-[8px] text-[13px] font-medium transition-all",
            showCreate ? "bg-surface-lighter text-text-muted border border-border" : "bg-primary text-white hover:bg-primary-dark")}>
          <Plus size={15} /> 新建任务
        </button>
      </div>

      {/* 可用 Provider */}
      <div className="flex gap-2 flex-wrap">
        {providers.map((p) => (
          <span key={p.id} className="text-[11px] px-2.5 py-1 rounded-full bg-success/8 text-success border border-success/15 font-medium">
            {p.name} · {p.default_model}
          </span>
        ))}
        {providers.length === 0 && <span className="text-[12px] text-text-faint">未检测到可用的 API Key</span>}
      </div>

      {/* 创建面板 */}
      {showCreate && (
        <div className="card p-5 space-y-3">
          <input value={title} onChange={(e) => setTitle(e.target.value)} placeholder="任务标题（可选）"
            className="w-full bg-surface border border-border rounded-[8px] px-3.5 py-2.5 text-[13px] focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/20 transition" />
          <textarea value={prompt} onChange={(e) => setPrompt(e.target.value)} placeholder="输入 Prompt，系统会分发给选中的 AI 执行..." rows={4}
            className="w-full bg-surface border border-border rounded-[8px] px-3.5 py-2.5 text-[13px] focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary/20 transition resize-none leading-relaxed" />
          {/* Smart routing recommendation */}
          {showCreate && routing && !routingLoading && (
            <div className="bg-surface-lighter/50 rounded-[10px] p-3.5 border border-border-light space-y-2.5">
              <div className="flex items-center gap-2">
                <Sparkles size={13} className="text-primary" />
                <span className="text-[12px] font-medium text-text-muted">智能路由推荐</span>
                {(() => {
                  const cfg = taskTypeConfig[routing.task_type] || taskTypeConfig.chat;
                  const TIcon = cfg.icon;
                  return (
                    <span className={cn("text-[11px] px-2 py-0.5 rounded-full font-medium flex items-center gap-1", cfg.bg, cfg.color)}>
                      <TIcon size={11} />
                      {cfg.label}
                      <span className="opacity-60">{Math.round(routing.confidence * 100)}%</span>
                    </span>
                  );
                })()}
              </div>
              <div className="flex gap-2 overflow-x-auto">
                {routing.recommendations.slice(0, 3).map((rec, i) => (
                  <button key={rec.model_id}
                    onClick={() => {
                      const matchProvider = providers.find(p => p.id === rec.provider_id);
                      if (matchProvider) setSelectedProvider(matchProvider.id);
                    }}
                    className={cn(
                      "flex-1 min-w-0 bg-surface rounded-[8px] p-2.5 border text-left transition-all hover:border-primary/40 hover:shadow-sm",
                      selectedProvider === rec.provider_id ? "border-primary/50 ring-1 ring-primary/15" : "border-border-light"
                    )}>
                    <div className="flex items-center gap-1.5 mb-1">
                      {i === 0 && <span className="text-[10px] bg-primary/10 text-primary px-1.5 py-[1px] rounded font-medium">TOP</span>}
                      <span className="text-[12px] font-medium truncate">{rec.model_name}</span>
                    </div>
                    <div className="text-[11px] text-text-faint truncate">{rec.reason}</div>
                    <div className="flex items-center gap-2 mt-1.5 text-[10px] text-text-faint">
                      <span>Arena {rec.arena_score}</span>
                      <span>${rec.cost_per_m_input}/M</span>
                    </div>
                  </button>
                ))}
              </div>
            </div>
          )}
          {routingLoading && (
            <div className="flex items-center gap-2 text-[12px] text-text-faint py-1">
              <Loader2 size={12} className="animate-spin" />
              分析任务类型...
            </div>
          )}

          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-[12px] text-text-faint">分发到</span>
              <select value={selectedProvider} onChange={(e) => setSelectedProvider(e.target.value)}
                className="bg-surface border border-border rounded-[8px] px-3 py-1.5 text-[13px] focus:outline-none focus:border-primary">
                <option value="all">全部 Agent 并行</option>
                {providers.map((p) => <option key={p.id} value={p.id}>{p.name} ({p.default_model})</option>)}
              </select>
            </div>
            <button onClick={handleCreate} disabled={creating || !prompt.trim()}
              className="flex items-center gap-2 px-5 py-2 bg-primary hover:bg-primary-dark rounded-[8px] text-[13px] text-white font-medium transition-all disabled:opacity-40">
              {creating ? <Loader2 size={14} className="animate-spin" /> : <Send size={14} />}
              {selectedProvider === "all" ? "并行分发" : "发送"}
            </button>
          </div>
        </div>
      )}

      {/* 任务列表 */}
      {parentTasks.length > 0 ? (
        <div className="space-y-2.5">
          {parentTasks.map((task) => {
            const cfg = statusCfg[task.status] || statusCfg.pending;
            const Icon = cfg.icon;
            const isExp = expandedTask === task.id;
            const subs = subtasks[task.id] || [];

            return (
              <div key={task.id} className="card overflow-hidden">
                <div className="px-5 py-3.5 flex items-center gap-3 cursor-pointer hover:bg-surface-lighter/50 transition-colors"
                  onClick={() => task.task_type === "multi_agent" ? toggleExpand(task.id) : setExpandedTask(isExp ? null : task.id)}>
                  <div className={cn("p-1.5 rounded-[6px]", cfg.bg)}>
                    <Icon size={14} className={cn(cfg.color, task.status === "running" && "animate-spin")} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-[13px] font-medium">{task.title}</span>
                      {task.task_type === "multi_agent" && <span className="text-[10px] px-1.5 py-0.5 rounded bg-primary/8 text-primary font-medium">多 Agent</span>}
                      <span className={cn("text-[11px]", cfg.color)}>{cfg.label}</span>
                    </div>
                    <div className="text-[12px] text-text-faint mt-0.5 truncate">{task.prompt.slice(0, 80)}</div>
                  </div>
                  <div className="flex items-center gap-3 text-[11px] text-text-faint shrink-0">
                    {task.input_tokens + task.output_tokens > 0 && <span className="flex items-center gap-0.5"><Cpu size={11} />{formatNumber(task.input_tokens + task.output_tokens)}</span>}
                    {task.estimated_cost > 0 && <span className="flex items-center gap-0.5"><DollarSign size={11} />${task.estimated_cost.toFixed(4)}</span>}
                    {task.latency_ms > 0 && <span className="flex items-center gap-0.5"><Timer size={11} />{(task.latency_ms / 1000).toFixed(1)}s</span>}
                  </div>
                  {task.status === "completed" && task.task_type !== "multi_agent" && (
                    <button onClick={(e) => { e.stopPropagation(); handleVerify(task.id); }}
                      disabled={verifying === task.id}
                      className="flex items-center gap-1 px-2.5 py-1 rounded-[6px] text-[11px] text-primary bg-primary/8 hover:bg-primary/12 font-medium transition-colors shrink-0">
                      {verifying === task.id ? <Loader2 size={11} className="animate-spin" /> : <ShieldCheck size={11} />}
                      验证
                    </button>
                  )}
                  {task.task_type === "multi_agent" && (isExp ? <ChevronUp size={14} className="text-text-faint" /> : <ChevronDown size={14} className="text-text-faint" />)}
                </div>

                {/* 多 Agent 子任务 */}
                {isExp && task.task_type === "multi_agent" && subs.length > 0 && (
                  <div className="border-t border-border-light px-5 py-3 space-y-2 bg-surface-lighter/30">
                    <div className="text-[11px] text-text-faint mb-1">Agent 工作状态</div>
                    {subs.map((sub) => {
                      const sc = statusCfg[sub.status] || statusCfg.pending;
                      const SI = sc.icon;
                      return (
                        <div key={sub.id} className="bg-surface-light rounded-[8px] p-3 border border-border-light">
                          <div className="flex items-center justify-between mb-1">
                            <div className="flex items-center gap-2">
                              <SI size={12} className={cn(sc.color, sub.status === "running" && "animate-spin")} />
                              <span className="text-[12px] font-medium">{sub.provider_id}</span>
                              <code className="text-[11px] text-text-faint bg-surface-lighter px-1.5 py-0.5 rounded">{sub.model}</code>
                              <span className={cn("text-[11px]", sc.color)}>{sc.label}</span>
                            </div>
                            <div className="flex items-center gap-2 text-[11px] text-text-faint">
                              {sub.input_tokens + sub.output_tokens > 0 && <span>{formatNumber(sub.input_tokens + sub.output_tokens)}t</span>}
                              {sub.estimated_cost > 0 && <span>${sub.estimated_cost.toFixed(4)}</span>}
                              {sub.latency_ms > 0 && <span>{(sub.latency_ms / 1000).toFixed(1)}s</span>}
                            </div>
                          </div>
                          {sub.status === "completed" && sub.result && <pre className="text-[11px] text-text-muted bg-surface rounded-[6px] p-2 mt-1 max-h-32 overflow-auto whitespace-pre-wrap">{sub.result.slice(0, 500)}{sub.result.length > 500 ? "..." : ""}</pre>}
                          {sub.status === "failed" && sub.error_msg && <div className="text-[11px] text-danger bg-danger/5 rounded-[6px] p-2 mt-1">{sub.error_msg.slice(0, 200)}</div>}
                        </div>
                      );
                    })}
                  </div>
                )}

                {/* 单任务展开 */}
                {isExp && task.task_type !== "multi_agent" && (
                  <div className="border-t border-border-light px-5 py-3 bg-surface-lighter/30 space-y-2">
                    {routeDecisions[task.id] && (() => {
                      const rd = routeDecisions[task.id]!;
                      const typeCfg = taskTypeConfig[rd.prompt_type] || taskTypeConfig.chat;
                      const TI = typeCfg.icon;
                      return (
                        <div className="flex items-center gap-2 text-[11px] text-text-faint bg-surface-lighter rounded-[6px] px-3 py-1.5">
                          <Sparkles size={11} className="text-primary" />
                          <span className={cn("flex items-center gap-1 px-1.5 py-0.5 rounded", typeCfg.bg, typeCfg.color)}><TI size={10} />{typeCfg.label} {Math.round(rd.confidence * 100)}%</span>
                          <span>推荐 <strong>{rd.recommended_model || "—"}</strong></span>
                          <span>→</span>
                          <span>实际 <strong>{task.model || rd.actual_model || task.provider_id}</strong></span>
                        </div>
                      );
                    })()}
                    {task.status === "completed" && task.result && <pre className="text-[12px] text-text-muted whitespace-pre-wrap max-h-48 overflow-auto">{task.result}</pre>}
                    {task.status === "failed" && task.error_msg && <div className="text-[12px] text-danger">{task.error_msg}</div>}
                    {task.status === "running" && <div className="flex items-center gap-2 text-[13px] text-primary"><Loader2 size={13} className="animate-spin" />执行中...</div>}
                  </div>
                )}

                {/* Verification results */}
                {isExp && subs.filter(s => s.task_type === "verification").length > 0 && (
                  <div className="border-t border-border-light px-5 py-3 bg-primary/3">
                    <div className="flex items-center gap-2 mb-2">
                      <ShieldCheck size={13} className="text-primary" />
                      <span className="text-[12px] font-medium text-primary">质量验证结果</span>
                    </div>
                    {subs.filter(s => s.task_type === "verification").map(v => (
                      <pre key={v.id} className="text-[11px] text-text-muted whitespace-pre-wrap max-h-40 overflow-auto bg-surface-light rounded-[6px] p-2">
                        {v.result || v.error_msg || "验证中..."}
                      </pre>
                    ))}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      ) : !showCreate && (
        <div className="flex flex-col items-center py-16 text-text-faint">
          <p className="text-[14px]">暂无任务</p>
          <p className="text-[12px] mt-1">点击"新建任务"开始使用</p>
        </div>
      )}
    </div>
  );
}
