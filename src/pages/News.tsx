import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Newspaper, RefreshCw, Loader2, ExternalLink, Rocket, TrendingUp, DollarSign, Wrench, MessageSquare, Filter } from "lucide-react";
import { cn } from "../lib/utils";

interface NewsItem {
  id: string;
  title: string;
  url: string;
  source: string;
  summary: string;
  timestamp: number;
  score: number;
  category: string;
}

interface NewsResult {
  items: NewsItem[];
  fetched_at: string;
  errors: string[];
}

const categoryConfig: Record<string, { icon: typeof Rocket; label: string; color: string; bg: string }> = {
  release: { icon: Rocket, label: "发布", color: "text-primary", bg: "bg-primary/10" },
  benchmark: { icon: TrendingUp, label: "评测", color: "text-warning", bg: "bg-warning/10" },
  pricing: { icon: DollarSign, label: "定价", color: "text-success", bg: "bg-success/10" },
  tool: { icon: Wrench, label: "工具", color: "text-accent", bg: "bg-accent/10" },
  discussion: { icon: MessageSquare, label: "讨论", color: "text-text-muted", bg: "bg-surface-lighter" },
};

function timeAgo(ts: number) {
  const d = Math.floor((Date.now() - ts) / 1000);
  if (d < 60) return "刚刚";
  if (d < 3600) return Math.floor(d / 60) + " 分钟前";
  if (d < 86400) return Math.floor(d / 3600) + " 小时前";
  if (d < 604800) return Math.floor(d / 86400) + " 天前";
  return new Date(ts).toLocaleDateString("zh-CN");
}

export default function News() {
  const [items, setItems] = useState<NewsItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [fetchedAt, setFetchedAt] = useState("");
  const [errors, setErrors] = useState<string[]>([]);
  const [filter, setFilter] = useState<string>("all");

  async function load() {
    setLoading(true);
    try {
      const result = await invoke<NewsResult>("fetch_news");
      setItems(result.items);
      setFetchedAt(result.fetched_at);
      setErrors(result.errors);
    } catch (e) { console.error(e); }
    setLoading(false);
  }

  useEffect(() => { load(); }, []);

  const filtered = filter === "all" ? items : items.filter(i => i.category === filter);
  const categories = Array.from(new Set(items.map(i => i.category)));

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-[22px] font-semibold tracking-tight">AI 动态</h1>
          <p className="text-[13px] text-text-muted mt-0.5">
            来自 HackerNews、LocalLLaMA、Singularity 的最新 AI 讨论
            {fetchedAt && <span className="ml-2 text-text-faint">· 更新于 {fetchedAt}</span>}
          </p>
        </div>
        <button onClick={load} disabled={loading}
          className="flex items-center gap-1.5 px-3.5 py-2 rounded-[8px] text-[13px] text-primary bg-primary/8 hover:bg-primary/12 font-medium transition-colors disabled:opacity-60">
          {loading ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
          {loading ? "抓取中" : "刷新"}
        </button>
      </div>

      {errors.length > 0 && (
        <div className="card px-4 py-2.5 border-warning/30 bg-warning/5">
          <div className="text-[11px] text-warning font-medium mb-1">部分数据源失败：</div>
          {errors.map((e, i) => <div key={i} className="text-[11px] text-text-muted">· {e}</div>)}
        </div>
      )}

      {/* Category filter */}
      <div className="flex items-center gap-2 flex-wrap">
        <Filter size={13} className="text-text-faint" />
        <button onClick={() => setFilter("all")}
          className={cn("text-[12px] px-3 py-1 rounded-full transition-colors",
            filter === "all" ? "bg-primary/10 text-primary font-medium" : "text-text-muted hover:bg-surface-lighter")}>
          全部 ({items.length})
        </button>
        {categories.map(cat => {
          const cfg = categoryConfig[cat];
          if (!cfg) return null;
          const count = items.filter(i => i.category === cat).length;
          const Icon = cfg.icon;
          return (
            <button key={cat} onClick={() => setFilter(cat)}
              className={cn("text-[12px] px-3 py-1 rounded-full transition-colors flex items-center gap-1.5",
                filter === cat ? cn(cfg.bg, cfg.color, "font-medium") : "text-text-muted hover:bg-surface-lighter")}>
              <Icon size={11} /> {cfg.label} ({count})
            </button>
          );
        })}
      </div>

      {/* News list */}
      {loading && items.length === 0 ? (
        <div className="card p-12 text-center">
          <Loader2 size={24} className="animate-spin text-primary mx-auto mb-3" />
          <p className="text-[13px] text-text-faint">正在抓取最新动态...</p>
        </div>
      ) : filtered.length > 0 ? (
        <div className="space-y-2">
          {filtered.map(item => {
            const cfg = categoryConfig[item.category] || categoryConfig.discussion;
            const Icon = cfg.icon;
            return (
              <button key={item.id}
                onClick={() => openUrl(item.url).catch(() => window.open(item.url, "_blank"))}
                className="card px-5 py-4 w-full text-left hover:shadow-md hover:-translate-y-0.5 transition-all group">
                <div className="flex items-start gap-3">
                  <div className={cn("p-1.5 rounded-[6px] shrink-0 mt-0.5", cfg.bg)}>
                    <Icon size={14} className={cfg.color} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-start justify-between gap-3 mb-1">
                      <h3 className="text-[14px] font-medium group-hover:text-primary transition-colors leading-snug flex-1">
                        {item.title}
                      </h3>
                      <div className="flex items-center gap-2 text-[11px] text-text-faint shrink-0">
                        <span className="font-mono">▲ {item.score}</span>
                        <ExternalLink size={11} className="opacity-0 group-hover:opacity-100 transition-opacity" />
                      </div>
                    </div>
                    {item.summary && (
                      <p className="text-[12px] text-text-muted leading-relaxed line-clamp-2 mb-1.5">
                        {item.summary}
                      </p>
                    )}
                    <div className="flex items-center gap-2.5 text-[11px] text-text-faint">
                      <span className="font-medium">{item.source}</span>
                      <span>·</span>
                      <span>{timeAgo(item.timestamp)}</span>
                      <span>·</span>
                      <span className={cfg.color}>{cfg.label}</span>
                    </div>
                  </div>
                </div>
              </button>
            );
          })}
        </div>
      ) : (
        <div className="card p-12 text-center">
          <Newspaper size={28} className="text-text-faint mx-auto mb-3" />
          <p className="text-[13px] text-text-muted">暂无 {filter === "all" ? "" : categoryConfig[filter]?.label} 相关动态</p>
        </div>
      )}
    </div>
  );
}
