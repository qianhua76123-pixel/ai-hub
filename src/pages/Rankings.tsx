import { useEffect, useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { cn } from "../lib/utils";
import { RefreshCw, Trophy, Code2, Eye, Sparkles, Filter, ArrowUpDown, ChevronDown } from "lucide-react";

// ── Types ───────────────────────────────────────────────────

interface RankedModel {
  rank: number;
  name: string;
  provider: string;
  score: number;
  source: string;
  category: string;
  votes: number;
  ci: number;
  license: string;
}

interface RankingsResult {
  arena_text: RankedModel[];
  arena_code: RankedModel[];
  arena_vision: RankedModel[];
  artificial_analysis: RankedModel[];
  fetched_at: string;
  errors: string[];
}

// ── Constants ───────────────────────────────────────────────

type TabKey = "arena_text" | "arena_code" | "arena_vision" | "artificial_analysis";

const TABS: { key: TabKey; label: string; icon: typeof Trophy; color: string }[] = [
  { key: "arena_text", label: "Arena 综合", icon: Trophy, color: "#0d9488" },
  { key: "arena_code", label: "Arena 编程", icon: Code2, color: "#0ea5e9" },
  { key: "arena_vision", label: "Arena 视觉", icon: Eye, color: "#8b5cf6" },
  { key: "artificial_analysis", label: "AA 质量指数", icon: Sparkles, color: "#f59e0b" },
];

const PROVIDER_COLORS: Record<string, string> = {
  Anthropic: "#d97706",
  OpenAI: "#10b981",
  Google: "#3b82f6",
  xAI: "#6366f1",
  DeepSeek: "#0d9488",
  Meta: "#0ea5e9",
  Alibaba: "#ef4444",
  Mistral: "#f97316",
  Moonshot: "#8b5cf6",
  "Zhipu AI": "#14b8a6",
};

const CATEGORY_LABELS: Record<string, string> = {
  flagship: "旗舰",
  reasoning: "推理",
  fast: "快速",
  "open-source": "开源",
  general: "通用",
};

// ── Component ───────────────────────────────────────────────

export default function Rankings() {
  const [data, setData] = useState<RankingsResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [activeTab, setActiveTab] = useState<TabKey>("arena_text");
  const [providerFilter, setProviderFilter] = useState<string>("all");
  const [categoryFilter, setCategoryFilter] = useState<string>("all");
  const [sortField, setSortField] = useState<"rank" | "score" | "name">("rank");
  const [sortAsc, setSortAsc] = useState(true);
  const [showFilterPanel, setShowFilterPanel] = useState(false);

  const fetchData = async () => {
    setLoading(true);
    try {
      const result = await invoke<RankingsResult>("fetch_rankings", { aaApiKey: null });
      setData(result);
    } catch (e) {
      console.error("fetch_rankings failed:", e);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => { fetchData(); }, []);

  // Current tab data
  const models = useMemo(() => {
    if (!data) return [];
    return data[activeTab] || [];
  }, [data, activeTab]);

  // Unique providers and categories for filters
  const providers = useMemo(() => {
    const set = new Set(models.map((m) => m.provider));
    return Array.from(set).sort();
  }, [models]);

  const categories = useMemo(() => {
    const set = new Set(models.map((m) => m.category));
    return Array.from(set).sort();
  }, [models]);

  // Filtered + sorted
  const displayed = useMemo(() => {
    let list = [...models];
    if (providerFilter !== "all") list = list.filter((m) => m.provider === providerFilter);
    if (categoryFilter !== "all") list = list.filter((m) => m.category === categoryFilter);
    list.sort((a, b) => {
      let cmp = 0;
      if (sortField === "rank") cmp = a.rank - b.rank;
      else if (sortField === "score") cmp = b.score - a.score;
      else cmp = a.name.localeCompare(b.name);
      return sortAsc ? cmp : -cmp;
    });
    return list;
  }, [models, providerFilter, categoryFilter, sortField, sortAsc]);

  // Max score for bar width
  const maxScore = useMemo(() => Math.max(...models.map((m) => m.score), 1), [models]);

  function handleSort(field: "rank" | "score" | "name") {
    if (sortField === field) setSortAsc(!sortAsc);
    else { setSortField(field); setSortAsc(field === "rank"); }
  }

  function medalEmoji(rank: number) {
    if (rank === 1) return "🥇";
    if (rank === 2) return "🥈";
    if (rank === 3) return "🥉";
    return null;
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-[22px] font-semibold tracking-tight text-text">模型排行榜</h1>
          <p className="text-[13px] text-text-muted mt-0.5">
            实时获取主流 AI 模型评测排名
            {data && <span className="text-text-faint ml-2">· 更新于 {data.fetched_at}</span>}
          </p>
        </div>
        <button
          onClick={fetchData}
          disabled={loading}
          className={cn(
            "flex items-center gap-1.5 px-3 py-1.5 rounded-[8px] text-[12px] font-medium transition-all",
            "bg-primary/10 text-primary hover:bg-primary/15",
            loading && "opacity-60 cursor-not-allowed"
          )}
        >
          <RefreshCw size={13} className={cn(loading && "animate-spin")} />
          {loading ? "获取中…" : "刷新"}
        </button>
      </div>

      {/* Errors */}
      {data && data.errors.length > 0 && (
        <div className="bg-warning/5 border border-warning/20 rounded-[10px] px-4 py-2.5 text-[12px] text-warning">
          {data.errors.filter(e => !e.includes("not configured")).map((e, i) => <div key={i}>{e}</div>)}
        </div>
      )}

      {/* Tabs */}
      <div className="flex items-center gap-1 p-1 bg-surface-lighter rounded-[10px]">
        {TABS.map(({ key, label, icon: Icon, color }) => {
          const count = data ? (data[key]?.length || 0) : 0;
          return (
            <button
              key={key}
              onClick={() => { setActiveTab(key); setProviderFilter("all"); setCategoryFilter("all"); }}
              className={cn(
                "flex items-center gap-1.5 px-3 py-2 rounded-[8px] text-[12px] font-medium transition-all flex-1 justify-center",
                activeTab === key
                  ? "bg-white dark:bg-surface shadow-sm text-text"
                  : "text-text-muted hover:text-text hover:bg-white/50"
              )}
            >
              <Icon size={13} style={activeTab === key ? { color } : undefined} />
              {label}
              {count > 0 && <span className="text-[10px] text-text-faint ml-0.5">({count})</span>}
            </button>
          );
        })}
      </div>

      {/* Filters */}
      <div className="flex items-center gap-3">
        <button
          onClick={() => setShowFilterPanel(!showFilterPanel)}
          className={cn(
            "flex items-center gap-1.5 px-2.5 py-1.5 rounded-[7px] text-[12px] transition-colors",
            showFilterPanel ? "bg-primary/10 text-primary" : "text-text-muted hover:bg-surface-lighter"
          )}
        >
          <Filter size={12} />
          筛选
          <ChevronDown size={11} className={cn("transition-transform", showFilterPanel && "rotate-180")} />
        </button>

        {(providerFilter !== "all" || categoryFilter !== "all") && (
          <button
            onClick={() => { setProviderFilter("all"); setCategoryFilter("all"); }}
            className="text-[11px] text-primary hover:underline"
          >
            清除筛选
          </button>
        )}

        <div className="flex-1" />

        <span className="text-[11px] text-text-faint">
          {displayed.length} / {models.length} 个模型
        </span>
      </div>

      {showFilterPanel && (
        <div className="flex gap-4 px-1">
          <div>
            <label className="text-[11px] text-text-faint mb-1 block">厂商</label>
            <select
              value={providerFilter}
              onChange={(e) => setProviderFilter(e.target.value)}
              className="text-[12px] bg-surface-lighter border border-border-light rounded-[7px] px-2 py-1.5 text-text min-w-[120px]"
            >
              <option value="all">全部</option>
              {providers.map((p) => <option key={p} value={p}>{p}</option>)}
            </select>
          </div>
          <div>
            <label className="text-[11px] text-text-faint mb-1 block">类型</label>
            <select
              value={categoryFilter}
              onChange={(e) => setCategoryFilter(e.target.value)}
              className="text-[12px] bg-surface-lighter border border-border-light rounded-[7px] px-2 py-1.5 text-text min-w-[120px]"
            >
              <option value="all">全部</option>
              {categories.map((c) => <option key={c} value={c}>{CATEGORY_LABELS[c] || c}</option>)}
            </select>
          </div>
        </div>
      )}

      {/* Table */}
      {loading && !data ? (
        <div className="card p-12 flex items-center justify-center">
          <div className="flex flex-col items-center gap-3">
            <RefreshCw size={24} className="animate-spin text-primary/60" />
            <span className="text-[13px] text-text-muted">正在获取排行榜数据…</span>
          </div>
        </div>
      ) : displayed.length === 0 ? (
        <div className="card p-12 text-center text-[13px] text-text-faint">
          {activeTab === "artificial_analysis"
            ? "需要在设置中配置 Artificial Analysis API Key"
            : "暂无数据"}
        </div>
      ) : (
        <div className="card overflow-hidden">
          {/* Table header */}
          <div className="grid grid-cols-[50px_1fr_120px_100px_90px_80px] gap-2 px-5 py-3 bg-surface-lighter/60 border-b border-border-light text-[11px] text-text-faint font-medium">
            <button className="flex items-center gap-1 hover:text-text" onClick={() => handleSort("rank")}>
              排名 {sortField === "rank" && <ArrowUpDown size={10} />}
            </button>
            <button className="flex items-center gap-1 hover:text-text" onClick={() => handleSort("name")}>
              模型 {sortField === "name" && <ArrowUpDown size={10} />}
            </button>
            <span>厂商</span>
            <button className="flex items-center gap-1 hover:text-text" onClick={() => handleSort("score")}>
              分数 {sortField === "score" && <ArrowUpDown size={10} />}
            </button>
            <span>类型</span>
            <span>投票</span>
          </div>

          {/* Rows */}
          <div className="divide-y divide-border-light/50">
            {displayed.map((m, i) => {
              const pColor = PROVIDER_COLORS[m.provider] || "#6b7280";
              const barWidth = Math.max(8, (m.score / maxScore) * 100);
              const isTop3 = m.rank <= 3;

              return (
                <div
                  key={`${m.source}-${m.name}-${i}`}
                  className={cn(
                    "grid grid-cols-[50px_1fr_120px_100px_90px_80px] gap-2 px-5 py-3 items-center transition-colors hover:bg-surface-lighter/40",
                    isTop3 && "bg-primary/[0.02]"
                  )}
                >
                  {/* Rank */}
                  <div className="flex items-center gap-1">
                    {medalEmoji(m.rank) ? (
                      <span className="text-[16px]">{medalEmoji(m.rank)}</span>
                    ) : (
                      <span className={cn("text-[13px] font-medium", isTop3 ? "text-primary" : "text-text-muted")}>
                        {m.rank}
                      </span>
                    )}
                  </div>

                  {/* Model name */}
                  <div className="min-w-0">
                    <div className="text-[13px] font-medium text-text truncate">{m.name}</div>
                    {m.ci > 0 && (
                      <span className="text-[10px] text-text-faint">±{m.ci}</span>
                    )}
                  </div>

                  {/* Provider */}
                  <div className="flex items-center gap-1.5">
                    <div className="w-[6px] h-[6px] rounded-full shrink-0" style={{ backgroundColor: pColor }} />
                    <span className="text-[12px] text-text-muted truncate">{m.provider}</span>
                  </div>

                  {/* Score with bar */}
                  <div className="space-y-1">
                    <span className={cn("text-[13px] font-semibold tabular-nums", isTop3 ? "text-primary" : "text-text")}>
                      {m.score.toLocaleString()}
                    </span>
                    <div className="w-full h-1 bg-surface-lighter rounded-full overflow-hidden">
                      <div
                        className="h-full rounded-full transition-all"
                        style={{ width: `${barWidth}%`, backgroundColor: pColor, opacity: 0.5 }}
                      />
                    </div>
                  </div>

                  {/* Category */}
                  <span className={cn(
                    "text-[11px] px-2 py-0.5 rounded-full w-fit",
                    m.category === "flagship" ? "bg-primary/8 text-primary" :
                    m.category === "reasoning" ? "bg-accent/8 text-accent" :
                    m.category === "fast" ? "bg-success/8 text-success" :
                    m.category === "open-source" ? "bg-warning/8 text-warning" :
                    "bg-surface-lighter text-text-faint"
                  )}>
                    {CATEGORY_LABELS[m.category] || m.category}
                  </span>

                  {/* Votes */}
                  <span className="text-[12px] text-text-faint tabular-nums">
                    {m.votes > 0 ? (m.votes >= 1000 ? `${(m.votes / 1000).toFixed(1)}k` : m.votes) : "—"}
                  </span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}
