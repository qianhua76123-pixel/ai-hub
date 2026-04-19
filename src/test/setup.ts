import "@testing-library/jest-dom/vitest";

// Mock Tauri invoke API
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    const mocks: Record<string, unknown> = {
      scan_providers: [],
      get_total_stats: { requests: 0, tokens: 0, cost: 0 },
      get_recent_traffic: [],
      get_daily_usage: [],
      get_rate_limit_status: [],
      get_app_info: { name: "AI Hub", version: "0.2.0", platform: "macos", arch: "aarch64", currency_rate: 7.2 },
      get_available_providers: [],
      get_tasks: [],
      get_model_prices: [],
      get_subscription_plans: [],
      get_cost_comparison: { monthly_api_cost_usd: 0, monthly_api_cost_cny: 0, comparisons: [] },
      get_pricing_info: { last_updated: "test", model_count: 0, source: "test" },
      get_subscription_roi: { results: [] },
      get_usage_by_provider: [],
      get_hourly_usage: [],
      get_usage_by_project: [],
      get_proxy_status: { running: true, port: 23456, base_url: "http://127.0.0.1:23456" },
      get_manageable_tools: [],
      get_env_exports: "# AI Hub exports",
      get_conversation_sources: [],
      get_recent_conversations: [],
      get_provider_presets: [],
      recommend_route: { task_type: "chat", confidence: 0.5, recommendations: [] },
    };
    return Promise.resolve(mocks[cmd] ?? null);
  }),
}));

// Mock localStorage
const store: Record<string, string> = {};
Object.defineProperty(window, "localStorage", {
  value: {
    getItem: (key: string) => store[key] ?? null,
    setItem: (key: string, value: string) => { store[key] = value; },
    removeItem: (key: string) => { delete store[key]; },
    clear: () => { Object.keys(store).forEach(k => delete store[k]); },
  },
});
