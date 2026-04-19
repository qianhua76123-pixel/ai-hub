import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Radar, Loader2, Plus, X, Search, Check, ExternalLink, Eye, EyeOff } from "lucide-react";
import { cn } from "../lib/utils";

interface DetectedProvider { id: string; name: string; status: string; detection_method: string; detection_detail: string; color: string; plan: string | null; }
interface ProviderPreset { id: string; name: string; category: string; api_format: string; base_url: string; env_key: string; default_model: string; models: string[]; color: string; description: string; doc_url: string; }

const ml: Record<string, string> = { env_var: "环境变量", config_file: "配置文件", ide_plugin: "IDE 插件", dotenv_file: ".env 文件" };
const catLabel: Record<string, string> = { international: "国际", china: "国内", aggregator: "聚合", cloud: "云平台" };

export default function Providers() {
  const [providers, setProviders] = useState<DetectedProvider[]>([]);
  const [scanning, setScanning] = useState(true);
  const [showAdd, setShowAdd] = useState(false);
  const [presets, setPresets] = useState<ProviderPreset[]>([]);
  const [presetSearch, setPresetSearch] = useState("");
  const [selectedPreset, setSelectedPreset] = useState<ProviderPreset | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [adding, setAdding] = useState(false);
  const [msg, setMsg] = useState("");

  useEffect(() => {
    invoke<DetectedProvider[]>("scan_providers").then((r) => { setProviders(r); setScanning(false); }).catch(() => setScanning(false));
  }, []);

  function rescan() {
    setScanning(true);
    invoke<DetectedProvider[]>("scan_providers").then((r) => { setProviders(r); setScanning(false); });
  }

  function openAddPanel() {
    setShowAdd(true);
    invoke<ProviderPreset[]>("get_provider_presets").then(setPresets);
  }

  async function handleAdd() {
    if (!selectedPreset || !apiKey.trim()) return;
    setAdding(true);
    try {
      const m = await invoke<string>("add_provider", { presetId: selectedPreset.id, apiKey: apiKey.trim() });
      setMsg(m); setApiKey(""); setSelectedPreset(null); setShowAdd(false);
      rescan();
      setTimeout(() => setMsg(""), 3000);
    } catch (e) { setMsg(String(e)); }
    setAdding(false);
  }

  const filteredPresets = presetSearch.trim()
    ? presets.filter((p) => (p.name + p.description + p.id).toLowerCase().includes(presetSearch.toLowerCase()))
    : presets;

  const groupedPresets = filteredPresets.reduce<Record<string, ProviderPreset[]>>((acc, p) => {
    if (!acc[p.category]) acc[p.category] = [];
    acc[p.category].push(p);
    return acc;
  }, {});

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-[22px] font-semibold tracking-tight">AI 工具</h1>
          <p className="text-[13px] text-text-muted mt-0.5">系统中检测到的 AI 工具与服务</p>
        </div>
        <div className="flex items-center gap-2">
          <button onClick={openAddPanel}
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-[8px] text-[13px] text-white bg-primary hover:bg-primary-dark font-medium transition-colors">
            <Plus size={14} /> 添加 Provider
          </button>
          <button onClick={rescan}
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-[8px] text-[13px] text-primary bg-primary/8 hover:bg-primary/12 font-medium transition-colors">
            <Radar size={14} className={scanning ? "animate-spin" : ""} />
            {scanning ? "扫描中" : "重新扫描"}
          </button>
        </div>
      </div>

      {msg && <div className="card px-4 py-3 text-[13px] text-primary bg-primary/5">{msg}</div>}

      {showAdd && (
        <div className="card p-5 space-y-4">
          <div className="flex items-center justify-between">
            <h2 className="text-[14px] font-medium">添加 AI Provider</h2>
            <button onClick={() => { setShowAdd(false); setSelectedPreset(null); setApiKey(""); }} className="text-text-faint hover:text-text-muted"><X size={16} /></button>
          </div>

          {!selectedPreset ? (
            <>
              <div className="flex items-center gap-2 bg-surface border border-border rounded-[8px] px-3 py-2 focus-within:border-primary transition">
                <Search size={14} className="text-text-faint" />
                <input value={presetSearch} onChange={(e) => setPresetSearch(e.target.value)}
                  placeholder="搜索 Provider..."
                  className="flex-1 bg-transparent text-[13px] outline-none placeholder:text-text-faint" />
              </div>
              <div className="max-h-[360px] overflow-auto space-y-4">
                {Object.entries(groupedPresets).map(([cat, items]) => (
                  <div key={cat}>
                    <div className="text-[11px] text-text-faint font-medium uppercase tracking-wider mb-2">{catLabel[cat] || cat}</div>
                    <div className="grid grid-cols-3 gap-2">
                      {items.map((p) => (
                        <button key={p.id} onClick={() => setSelectedPreset(p)}
                          className="text-left px-3 py-2.5 rounded-[8px] border border-border-light hover:border-primary/30 hover:bg-primary/3 transition-all">
                          <div className="flex items-center gap-2 mb-0.5">
                            <div className="w-6 h-6 rounded-[6px] flex items-center justify-center text-white text-[10px] font-semibold"
                              style={{ backgroundColor: p.color }}>{p.name[0]}</div>
                            <span className="text-[13px] font-medium">{p.name}</span>
                          </div>
                          <div className="text-[11px] text-text-faint">{p.description}</div>
                        </button>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
              <div className="text-[12px] text-text-faint text-center">{presets.length} 个 Provider 可选</div>
            </>
          ) : (
            <div className="space-y-3">
              <div className="flex items-center gap-3 bg-surface-lighter rounded-[10px] p-3">
                <div className="w-10 h-10 rounded-[8px] flex items-center justify-center text-white font-semibold"
                  style={{ backgroundColor: selectedPreset.color }}>{selectedPreset.name[0]}</div>
                <div className="flex-1">
                  <div className="text-[14px] font-medium">{selectedPreset.name}</div>
                  <div className="text-[12px] text-text-faint">{selectedPreset.description}</div>
                </div>
                <button onClick={() => setSelectedPreset(null)} className="text-[12px] text-primary">换一个</button>
              </div>
              <div>
                <label className="text-[12px] text-text-muted mb-1.5 block">API Key ({selectedPreset.env_key})</label>
                <div className="flex items-center bg-surface border border-border rounded-[8px] px-3 py-2.5 focus-within:border-primary focus-within:ring-1 focus-within:ring-primary/20 transition">
                  <input value={apiKey} onChange={(e) => setApiKey(e.target.value)}
                    type={showKey ? "text" : "password"}
                    placeholder={`输入 ${selectedPreset.env_key}`}
                    className="flex-1 bg-transparent text-[13px] outline-none font-mono placeholder:text-text-faint" />
                  <button onClick={() => setShowKey(!showKey)} className="text-text-faint hover:text-text-muted ml-2">
                    {showKey ? <EyeOff size={14} /> : <Eye size={14} />}
                  </button>
                </div>
              </div>
              <div className="text-[11px] text-text-faint">
                模型: {selectedPreset.models.join(", ")}
                {selectedPreset.doc_url && (
                  <> · <a href={selectedPreset.doc_url} target="_blank" rel="noopener noreferrer" className="text-primary hover:underline inline-flex items-center gap-0.5">文档 <ExternalLink size={10} /></a></>
                )}
              </div>
              <div className="flex justify-end gap-2">
                <button onClick={() => { setSelectedPreset(null); setApiKey(""); }}
                  className="px-4 py-2 rounded-[8px] text-[13px] text-text-muted hover:bg-surface-lighter transition-colors">取消</button>
                <button onClick={handleAdd} disabled={!apiKey.trim() || adding}
                  className="flex items-center gap-2 px-5 py-2 bg-primary hover:bg-primary-dark rounded-[8px] text-[13px] text-white font-medium transition-all disabled:opacity-40">
                  {adding ? <Loader2 size={14} className="animate-spin" /> : <Check size={14} />}
                  添加
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {scanning && providers.length === 0 ? (
        <div className="flex flex-col items-center py-20 text-text-faint">
          <Loader2 size={24} className="animate-spin mb-3 text-primary" />
          <span className="text-[13px]">正在扫描系统...</span>
        </div>
      ) : providers.length === 0 ? (
        <div className="text-center py-20 text-text-faint text-[13px]">未检测到 AI 工具</div>
      ) : (
        <div className="space-y-2.5">
          {providers.map((p) => (
            <div key={p.id} className="card px-5 py-4 flex items-center gap-4">
              <div className="w-10 h-10 rounded-[10px] flex items-center justify-center text-white font-semibold shrink-0"
                style={{ backgroundColor: p.color }}>{p.name[0]}</div>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-0.5">
                  <span className="text-[14px] font-medium">{p.name}</span>
                  <span className={cn("text-[11px] px-2 py-[2px] rounded-full font-medium",
                    p.status === "connected" ? "bg-success/10 text-success" : "bg-warning/10 text-warning")}>
                    {p.status === "connected" ? "已连接" : p.status}
                  </span>
                </div>
                <p className="text-[12px] text-text-muted">{p.detection_detail}</p>
                <span className="text-[11px] text-text-faint">{ml[p.detection_method] || p.detection_method}</span>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
