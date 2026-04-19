import { useEffect, useState, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Search, MessageSquare, RefreshCw, Loader2, Filter, Clock, Cpu, X } from "lucide-react";
import { cn, formatTokens } from "../lib/utils";

interface ConversationRecord {
  id: string; source: string; tool: string; title: string;
  content: string; timestamp: number; tokens: number; model: string;
}
interface SourceInfo { source: string; count: number; }

const sourceColors: Record<string, string> = {
  "Claude Code": "#d97706", "Cursor": "#00d4aa", "Codex CLI": "#10a37f",
};

export default function Conversations() {
  const [conversations, setConversations] = useState<ConversationRecord[]>([]);
  const [sources, setSources] = useState<SourceInfo[]>([]);
  const [query, setQuery] = useState("");
  const [sourceFilter, setSourceFilter] = useState("all");
  const [selectedConv, setSelectedConv] = useState<ConversationRecord | null>(null);
  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const searchRef = useRef<HTMLInputElement>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    invoke<SourceInfo[]>("get_conversation_sources").then(setSources);
    loadConversations();
  }, []);

  function loadConversations() {
    setLoading(true);
    if (query.trim()) {
      invoke<ConversationRecord[]>("search_conversations", { query, source: sourceFilter, limit: 100 })
        .then((r) => { setConversations(r); setLoading(false); }).catch(() => setLoading(false));
    } else {
      invoke<ConversationRecord[]>("get_recent_conversations", { limit: 100 })
        .then((r) => { setConversations(r); setLoading(false); }).catch(() => setLoading(false));
    }
  }

  const debouncedSearch = useCallback((q: string) => {
    clearTimeout(timerRef.current);
    timerRef.current = setTimeout(() => {
      setLoading(true);
      if (q.trim()) {
        invoke<ConversationRecord[]>("search_conversations", { query: q, source: sourceFilter, limit: 100 })
          .then((r) => { setConversations(r); setLoading(false); }).catch(() => setLoading(false));
      } else {
        invoke<ConversationRecord[]>("get_recent_conversations", { limit: 100 })
          .then((r) => { setConversations(r); setLoading(false); }).catch(() => setLoading(false));
      }
    }, 300);
  }, [sourceFilter]);

  // debouncedSearch already depends on sourceFilter via useCallback,
  // so this single effect covers both query and filter changes
  useEffect(() => { debouncedSearch(query); }, [query, debouncedSearch]);

  async function handleRefresh() {
    setRefreshing(true);
    await invoke("refresh_conversations");
    invoke<SourceInfo[]>("get_conversation_sources").then(setSources);
    loadConversations();
    setRefreshing(false);
  }

  function timeAgo(ts: number) {
    const d = Math.floor((Date.now() - ts) / 1000);
    if (d < 60) return "刚刚";
    if (d < 3600) return Math.floor(d / 60) + " 分钟前";
    if (d < 86400) return Math.floor(d / 3600) + " 小时前";
    if (d < 604800) return Math.floor(d / 86400) + " 天前";
    return new Date(ts).toLocaleDateString("zh-CN");
  }

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-[22px] font-semibold tracking-tight">对话搜索</h1>
          <p className="text-[13px] text-text-muted mt-0.5">跨工具搜索所有 AI 对话历史</p>
        </div>
        <button onClick={handleRefresh}
          className="flex items-center gap-1.5 px-3.5 py-2 rounded-[8px] text-[13px] text-primary bg-primary/8 hover:bg-primary/12 font-medium transition-colors">
          {refreshing ? <Loader2 size={14} className="animate-spin" /> : <RefreshCw size={14} />}
          {refreshing ? "扫描中" : "刷新"}
        </button>
      </div>

      <div className="card p-4 space-y-3">
        <div className="flex items-center gap-2.5">
          <div className="flex-1 flex items-center gap-2 bg-surface border border-border rounded-[8px] px-3.5 py-2.5 focus-within:border-primary focus-within:ring-1 focus-within:ring-primary/20 transition">
            <Search size={15} className="text-text-faint shrink-0" />
            <input ref={searchRef} value={query} onChange={(e) => setQuery(e.target.value)}
              placeholder="搜索对话内容、标题..."
              className="flex-1 bg-transparent text-[13px] outline-none placeholder:text-text-faint" />
            {query && <button onClick={() => setQuery("")} className="text-text-faint hover:text-text-muted"><X size={14} /></button>}
          </div>
          {loading && <Loader2 size={16} className="animate-spin text-primary shrink-0" />}
        </div>
        <div className="flex items-center gap-2">
          <Filter size={13} className="text-text-faint" />
          <button onClick={() => setSourceFilter("all")}
            className={cn("text-[12px] px-2.5 py-1 rounded-full transition-colors",
              sourceFilter === "all" ? "bg-primary/10 text-primary font-medium" : "text-text-muted hover:bg-surface-lighter")}>
            全部
          </button>
          {sources.map((s) => (
            <button key={s.source} onClick={() => setSourceFilter(s.source)}
              className={cn("text-[12px] px-2.5 py-1 rounded-full transition-colors flex items-center gap-1.5",
                sourceFilter === s.source ? "bg-primary/10 text-primary font-medium" : "text-text-muted hover:bg-surface-lighter")}>
              <div className="w-2 h-2 rounded-full" style={{ backgroundColor: sourceColors[s.source] || "#666" }} />
              {s.source}
              <span className="text-text-faint text-[11px]">{s.count}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-5 gap-4" style={{ minHeight: 400 }}>
        <div className="col-span-2 space-y-1.5 max-h-[calc(100vh-320px)] overflow-auto pr-1">
          {conversations.length > 0 ? conversations.map((conv) => (
            <button key={conv.id} onClick={() => setSelectedConv(conv)}
              className={cn("w-full text-left px-4 py-3 rounded-[10px] transition-all",
                selectedConv?.id === conv.id
                  ? "bg-primary/8 border border-primary/20"
                  : "hover:bg-surface-lighter border border-transparent")}>
              <div className="flex items-center gap-2 mb-1">
                <div className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: sourceColors[conv.source] || "#666" }} />
                <span className="text-[13px] font-medium truncate flex-1">{conv.title}</span>
              </div>
              <div className="flex items-center gap-2 text-[11px] text-text-faint">
                <span>{conv.source}</span>
                {conv.model && <><span>·</span><span>{conv.model}</span></>}
                <span className="ml-auto">{timeAgo(conv.timestamp)}</span>
              </div>
            </button>
          )) : (
            <div className="flex flex-col items-center py-16 text-text-faint">
              <MessageSquare size={28} className="mb-3 opacity-30" />
              <p className="text-[13px]">{query ? "未找到匹配的对话" : "暂无对话记录"}</p>
              <p className="text-[11px] mt-1">使用 AI 工具后自动收录</p>
            </div>
          )}
        </div>

        <div className="col-span-3 card p-5 max-h-[calc(100vh-320px)] overflow-auto">
          {selectedConv ? (
            <div>
              <div className="flex items-center justify-between mb-4">
                <div>
                  <h2 className="text-[15px] font-medium">{selectedConv.title}</h2>
                  <div className="flex items-center gap-3 mt-1 text-[12px] text-text-faint">
                    <span className="flex items-center gap-1">
                      <div className="w-2 h-2 rounded-full" style={{ backgroundColor: sourceColors[selectedConv.source] || "#666" }} />
                      {selectedConv.source}
                    </span>
                    {selectedConv.model && <span className="flex items-center gap-1"><Cpu size={11} />{selectedConv.model}</span>}
                    {selectedConv.tokens > 0 && <span>{formatTokens(selectedConv.tokens)} tokens</span>}
                    <span className="flex items-center gap-1"><Clock size={11} />{timeAgo(selectedConv.timestamp)}</span>
                  </div>
                </div>
              </div>
              <div className="border-t border-border-light pt-4">
                <pre className="text-[12px] text-text-muted whitespace-pre-wrap leading-relaxed font-sans">
                  {selectedConv.content}
                </pre>
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-text-faint py-20">
              <Search size={28} className="mb-3 opacity-20" />
              <p className="text-[13px]">选择一个对话查看详情</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
