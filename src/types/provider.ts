export type ProviderStatus =
  | "connected"
  | "disconnected"
  | "error"
  | "detecting";

export interface ProviderCapability {
  type: "chat" | "code" | "image" | "audio" | "embedding" | "search";
  models: string[];
}

export interface AuthConfig {
  type: "api_key" | "oauth" | "cookie" | "local_config";
  value?: string;
  refreshToken?: string;
}

export interface UsageData {
  providerId: string;
  period: { start: Date; end: Date };
  totalTokens: number;
  inputTokens: number;
  outputTokens: number;
  requests: number;
  estimatedCost: number;
  breakdown: UsageBreakdown[];
}

export interface UsageBreakdown {
  model: string;
  tokens: number;
  requests: number;
  cost: number;
}

export interface QuotaInfo {
  type: "api_credits" | "subscription" | "free_tier";
  limit: number | null;
  used: number;
  resetsAt: Date | null;
  plan: string;
}

export interface BillingInfo {
  plan: string;
  pricePerMonth: number;
  nextBillingDate: Date | null;
  paymentMethod?: string;
}

export interface TaskHandle {
  id: string;
  providerId: string;
  model: string;
  status: TaskStatus;
  createdAt: Date;
  prompt: string;
  result?: string;
  tokensUsed?: number;
  cost?: number;
  duration?: number;
}

export type TaskStatus =
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "cancelled";

export interface DetectedProvider {
  id: string;
  name: string;
  icon: string;
  status: ProviderStatus;
  detection_method: string;
  detection_detail: string;
  color: string;
  capabilities: ProviderCapability[];
  auth?: AuthConfig;
  quota?: QuotaInfo;
  billing?: BillingInfo;
}

export interface TrafficRecord {
  id: string;
  timestamp: Date;
  providerId: string;
  model: string;
  endpoint: string;
  inputTokens: number;
  outputTokens: number;
  latencyMs: number;
  status: "success" | "error" | "rate_limited";
  estimatedCost: number;
  source: string;
}
