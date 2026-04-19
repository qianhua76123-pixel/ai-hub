import { useEffect, useState } from "react";
import { NavLink, Outlet } from "react-router-dom";
import { LayoutDashboard, Zap, BarChart3, CreditCard, Settings, Radar, Search, MessageSquare, Trophy, Sparkles } from "lucide-react";
import { cn } from "../lib/utils";
import CommandPalette from "./CommandPalette";
import Onboarding, { useOnboarding } from "./Onboarding";

const navItems = [
  { to: "/", icon: LayoutDashboard, label: "总览" },
  { to: "/providers", icon: Radar, label: "工具" },
  { to: "/tasks", icon: Zap, label: "任务" },
  { to: "/conversations", icon: MessageSquare, label: "对话" },
  { to: "/rankings", icon: Trophy, label: "排行" },
  { to: "/usage", icon: BarChart3, label: "用量" },
  { to: "/billing", icon: CreditCard, label: "订阅" },
  { to: "/advisor", icon: Sparkles, label: "顾问" },
  { to: "/settings", icon: Settings, label: "设置" },
];

export default function Layout() {
  const [cmdOpen, setCmdOpen] = useState(false);
  const { show: showOnboarding, dismiss: dismissOnboarding } = useOnboarding();
  useEffect(() => {
    const h = (e: KeyboardEvent) => { if ((e.metaKey || e.ctrlKey) && e.key === "k") { e.preventDefault(); setCmdOpen(v => !v); } };
    window.addEventListener("keydown", h); return () => window.removeEventListener("keydown", h);
  }, []);

  return (
    <div className="h-screen flex bg-gradient-scenic text-text">
      {/* Translucent sidebar */}
      <aside className="w-[200px] sidebar-glass border-r border-border/60 flex flex-col shrink-0">
        {/* Drag region / title bar area */}
        <div className="h-12 flex items-end px-4 pb-2" style={{ WebkitAppRegion: "drag" } as React.CSSProperties}>
          <div className="flex items-center gap-2">
            <div className="w-[22px] h-[22px] rounded-[6px] bg-gradient-to-br from-primary to-accent flex items-center justify-center">
              <span className="text-white text-[10px] font-bold">A</span>
            </div>
            <span className="text-[14px] font-semibold tracking-tight text-text">AI Hub</span>
          </div>
        </div>

        {/* Nav */}
        <nav className="flex-1 px-3 pt-4 space-y-0.5">
          {navItems.map(({ to, icon: Icon, label }) => (
            <NavLink key={to} to={to}
              className={({ isActive }) => cn(
                "flex items-center gap-2.5 px-2.5 py-[7px] rounded-[8px] text-[13px] transition-all duration-150",
                isActive
                  ? "bg-primary/10 text-primary font-medium"
                  : "text-text-muted hover:bg-black/[0.03] hover:text-text"
              )}>
              <Icon size={16} strokeWidth={1.6} />
              {label}
            </NavLink>
          ))}
        </nav>

        {/* Bottom search */}
        <div className="px-3 pb-3">
          <button onClick={() => setCmdOpen(true)}
            className="w-full flex items-center gap-2 px-2.5 py-[7px] rounded-[8px] text-[12px] text-text-faint hover:bg-black/[0.03] hover:text-text-muted transition-colors">
            <Search size={14} strokeWidth={1.6} />
            搜索
            <kbd className="ml-auto text-[10px] text-text-faint/70 bg-black/[0.04] px-1.5 py-[2px] rounded-[4px]">⌘K</kbd>
          </button>
        </div>
      </aside>

      {/* Main content */}
      <main className="flex-1 overflow-auto">
        <div className="max-w-[920px] mx-auto px-8 py-7">
          <Outlet />
        </div>
      </main>

      <CommandPalette open={cmdOpen} onClose={() => setCmdOpen(false)} />
      {showOnboarding && <Onboarding onDone={dismissOnboarding} />}
    </div>
  );
}
