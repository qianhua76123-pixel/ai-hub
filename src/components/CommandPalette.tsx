import { useEffect, useRef, useState, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import { Search, LayoutDashboard, Radar, Zap, BarChart3, CreditCard, Settings, ArrowRight, MessageSquare } from "lucide-react";
import { cn } from "../lib/utils";

interface Item { id: string; label: string; desc: string; icon: typeof Search; action: () => void; kw: string; }
interface Props { open: boolean; onClose: () => void; }

export default function CommandPalette({ open, onClose }: Props) {
  const [q, setQ] = useState("");
  const [idx, setIdx] = useState(0);
  const ref = useRef<HTMLInputElement>(null);
  const nav = useNavigate();

  const items: Item[] = [
    { id: "d", label: "总览", desc: "查看仪表盘", icon: LayoutDashboard, action: () => nav("/"), kw: "dashboard 总览" },
    { id: "p", label: "AI 工具", desc: "检测与管理", icon: Radar, action: () => nav("/providers"), kw: "providers 工具" },
    { id: "t", label: "任务中心", desc: "创建与执行", icon: Zap, action: () => nav("/tasks"), kw: "tasks 任务" },
    { id: "c", label: "对话搜索", desc: "跨工具搜索", icon: MessageSquare, action: () => nav("/conversations"), kw: "conversations 对话 搜索" },
    { id: "u", label: "用量统计", desc: "Token 与费用", icon: BarChart3, action: () => nav("/usage"), kw: "usage 用量" },
    { id: "b", label: "订阅管理", desc: "价格与对比", icon: CreditCard, action: () => nav("/billing"), kw: "billing 订阅" },
    { id: "s", label: "设置", desc: "代理与配置", icon: Settings, action: () => nav("/settings"), kw: "settings 设置" },
  ];

  const filtered = q.trim() ? items.filter(i => (i.label + i.desc + i.kw).toLowerCase().includes(q.toLowerCase())) : items;

  const onKey = useCallback((e: KeyboardEvent) => {
    if (!open) return;
    if (e.key === "Escape") onClose();
    else if (e.key === "ArrowDown") { e.preventDefault(); setIdx(i => (i + 1) % filtered.length); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setIdx(i => (i - 1 + filtered.length) % filtered.length); }
    else if (e.key === "Enter" && filtered[idx]) { filtered[idx].action(); onClose(); }
  }, [open, onClose, filtered, idx]);

  useEffect(() => { window.addEventListener("keydown", onKey); return () => window.removeEventListener("keydown", onKey); }, [onKey]);
  useEffect(() => { if (open) { setQ(""); setIdx(0); setTimeout(() => ref.current?.focus(), 50); } }, [open]);
  useEffect(() => { setIdx(0); }, [q]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-start justify-center pt-[18vh]">
      <div className="absolute inset-0 bg-black/20 backdrop-blur-[2px]" onClick={onClose} />
      <div className="relative w-full max-w-[440px] bg-surface-light rounded-[14px] border border-border shadow-xl overflow-hidden" style={{ boxShadow: "0 16px 70px rgba(0,0,0,0.12), 0 0 1px rgba(0,0,0,0.1)" }}>
        <div className="flex items-center gap-2.5 px-4 py-3 border-b border-border-light">
          <Search size={16} className="text-text-faint shrink-0" strokeWidth={1.6} />
          <input ref={ref} value={q} onChange={e => setQ(e.target.value)}
            placeholder="搜索页面、功能..."
            className="flex-1 bg-transparent text-[14px] outline-none placeholder:text-text-faint" />
        </div>
        <div className="max-h-[320px] overflow-auto p-1.5">
          {filtered.length > 0 ? filtered.map((item, i) => (
            <button key={item.id}
              onClick={() => { item.action(); onClose(); }}
              onMouseEnter={() => setIdx(i)}
              className={cn(
                "w-full flex items-center gap-3 px-3 py-2.5 rounded-[8px] text-left transition-colors",
                idx === i ? "bg-primary/8 text-text" : "text-text-muted hover:bg-surface-lighter"
              )}>
              <item.icon size={17} strokeWidth={1.5} className={idx === i ? "text-primary" : "text-text-faint"} />
              <div className="flex-1">
                <div className="text-[13px] font-medium">{item.label}</div>
                <div className="text-[11px] text-text-faint">{item.desc}</div>
              </div>
              {idx === i && <ArrowRight size={13} className="text-text-faint" />}
            </button>
          )) : <div className="py-8 text-center text-text-faint text-[13px]">未找到结果</div>}
        </div>
      </div>
    </div>
  );
}
