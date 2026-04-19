import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Sparkles, Shield, Zap, BarChart3, ChevronRight, Check, Radar, Loader2 } from "lucide-react";
import { cn } from "../lib/utils";

interface DetectedProvider { id: string; name: string; status: string; detection_detail: string; color: string; }

const ONBOARDING_KEY = "ai-hub-onboarding-done";

export function useOnboarding() {
  const [show, setShow] = useState(false);
  useEffect(() => {
    if (!localStorage.getItem(ONBOARDING_KEY)) setShow(true);
  }, []);
  return { show, dismiss: () => { localStorage.setItem(ONBOARDING_KEY, "1"); setShow(false); } };
}

const features = [
  { icon: Radar, title: "零配置自动检测", desc: "打开即用，自动发现本机所有 AI 工具和 API Key", color: "text-primary" },
  { icon: Shield, title: "本地优先隐私", desc: "所有数据存本地，API Key 加密保存，不上传不经过第三方", color: "text-success" },
  { icon: BarChart3, title: "精准费用追踪", desc: "区分 cache read/write/input/output，精确到每一次请求", color: "text-warning" },
  { icon: Zap, title: "智能任务路由", desc: "根据任务类型自动推荐最优模型，多 Agent 并行执行", color: "text-primary" },
];

export default function Onboarding({ onDone }: { onDone: () => void }) {
  const [step, setStep] = useState(0);
  const [providers, setProviders] = useState<DetectedProvider[]>([]);
  const [scanning, setScanning] = useState(false);

  async function handleScan() {
    setScanning(true);
    try {
      const result = await invoke<DetectedProvider[]>("scan_providers");
      setProviders(result);
    } catch (e) { console.error(e); }
    setScanning(false);
  }

  useEffect(() => {
    if (step === 1) handleScan();
  }, [step]);

  return (
    <div className="fixed inset-0 z-50 bg-black/40 flex items-center justify-center backdrop-blur-sm">
      <div className="bg-surface rounded-2xl shadow-2xl w-[560px] overflow-hidden border border-border">
        {/* Progress bar */}
        <div className="h-1 bg-surface-lighter">
          <div className="h-full bg-primary transition-all duration-500" style={{ width: `${((step + 1) / 3) * 100}%` }} />
        </div>

        {/* Step 0: Welcome */}
        {step === 0 && (
          <div className="p-8 space-y-6">
            <div className="text-center">
              <div className="w-14 h-14 rounded-2xl bg-gradient-to-br from-primary to-accent flex items-center justify-center mx-auto mb-4">
                <Sparkles size={28} className="text-white" />
              </div>
              <h1 className="text-2xl font-bold tracking-tight">欢迎使用 AI Hub</h1>
              <p className="text-[14px] text-text-muted mt-2">统一管理你的所有 AI 工具</p>
            </div>

            <div className="grid grid-cols-2 gap-3">
              {features.map((f) => (
                <div key={f.title} className="bg-surface-lighter rounded-xl p-4 border border-border-light">
                  <f.icon size={18} className={cn(f.color, "mb-2")} />
                  <div className="text-[13px] font-medium mb-0.5">{f.title}</div>
                  <div className="text-[11px] text-text-faint leading-relaxed">{f.desc}</div>
                </div>
              ))}
            </div>

            <button onClick={() => setStep(1)}
              className="w-full py-3 bg-primary hover:bg-primary-dark text-white rounded-xl text-[14px] font-medium transition-colors flex items-center justify-center gap-2">
              开始设置 <ChevronRight size={16} />
            </button>
          </div>
        )}

        {/* Step 1: Auto-detect */}
        {step === 1 && (
          <div className="p-8 space-y-5">
            <div>
              <h2 className="text-lg font-semibold">检测 AI 工具</h2>
              <p className="text-[13px] text-text-muted mt-1">正在扫描本机的 AI 工具和 API Key</p>
            </div>

            <div className="min-h-[200px]">
              {scanning ? (
                <div className="flex flex-col items-center justify-center py-10">
                  <Loader2 size={24} className="animate-spin text-primary mb-3" />
                  <span className="text-[13px] text-text-faint">扫描中...</span>
                </div>
              ) : providers.length > 0 ? (
                <div className="space-y-2">
                  {providers.map((p) => (
                    <div key={p.id} className="flex items-center gap-3 px-4 py-2.5 bg-surface-lighter rounded-lg">
                      <div className="w-8 h-8 rounded-lg flex items-center justify-center text-white text-[11px] font-semibold"
                        style={{ backgroundColor: p.color }}>{p.name[0]}</div>
                      <div className="flex-1">
                        <div className="text-[13px] font-medium">{p.name}</div>
                        <div className="text-[11px] text-text-faint">{p.detection_detail}</div>
                      </div>
                      <Check size={14} className="text-success" />
                    </div>
                  ))}
                  <div className="text-[12px] text-text-faint text-center pt-2">
                    检测到 {providers.length} 个 AI 工具
                  </div>
                </div>
              ) : (
                <div className="text-center py-10 text-text-faint">
                  <Radar size={24} className="mx-auto mb-2 opacity-30" />
                  <p className="text-[13px]">未检测到 AI 工具</p>
                  <p className="text-[11px] mt-1">你可以稍后在"工具"页面手动添加</p>
                </div>
              )}
            </div>

            <div className="flex gap-3">
              <button onClick={() => setStep(0)}
                className="px-5 py-2.5 rounded-xl text-[13px] text-text-muted hover:bg-surface-lighter transition-colors">返回</button>
              <button onClick={() => setStep(2)}
                className="flex-1 py-2.5 bg-primary hover:bg-primary-dark text-white rounded-xl text-[13px] font-medium transition-colors flex items-center justify-center gap-2">
                继续 <ChevronRight size={14} />
              </button>
            </div>
          </div>
        )}

        {/* Step 2: Ready */}
        {step === 2 && (
          <div className="p-8 space-y-5">
            <div className="text-center py-4">
              <div className="w-16 h-16 rounded-full bg-success/10 flex items-center justify-center mx-auto mb-4">
                <Check size={32} className="text-success" />
              </div>
              <h2 className="text-xl font-bold">设置完成</h2>
              <p className="text-[14px] text-text-muted mt-2">AI Hub 已就绪，开始管理你的 AI 工具吧</p>
            </div>

            <div className="bg-surface-lighter rounded-xl p-4 space-y-2 text-[12px] text-text-muted">
              <div className="flex items-start gap-2"><span className="text-primary mt-0.5">-</span>代理已自动启动，Claude Code 等工具的流量将被记录</div>
              <div className="flex items-start gap-2"><span className="text-primary mt-0.5">-</span>在"用量"页面查看费用趋势和模型使用明细</div>
              <div className="flex items-start gap-2"><span className="text-primary mt-0.5">-</span>在"任务"页面使用智能路由和多 Agent 并行</div>
              <div className="flex items-start gap-2"><span className="text-primary mt-0.5">-</span>在"订阅"页面查看 ROI 分析和模型评比</div>
            </div>

            <button onClick={onDone}
              className="w-full py-3 bg-primary hover:bg-primary-dark text-white rounded-xl text-[14px] font-medium transition-colors">
              进入 AI Hub
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
