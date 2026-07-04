export interface AgentProviderStatus {
  id: string;
  title: string;
  command: string;
  args: string[];
  available: boolean;
  path?: string | null;
  version?: string | null;
  error?: string | null;
}

export interface ProjectConfig {
  id: string;
  name: string;
  websiteUrl: string;
  path: string;
  createdAt: string;
  updatedAt: string;
}

export interface ContextDoc {
  key: string;
  fileName: string;
  title: string;
  content: string;
}

export interface RunState {
  id: string;
  kind: "initial_analysis" | "x_account_analysis";
  status: "running" | "completed" | "failed";
  providerId?: string | null;
  providerTitle?: string | null;
  externalSessionId?: string | null;
  codexThreadId?: string | null;
  startedAt: string;
  completedAt?: string | null;
  logPath: string;
  error?: string | null;
}

export interface RunActivity {
  kind: string;
  title: string;
  message: string;
}

export interface ChannelSetup {
  id: string;
  name: string;
  status: "not_configured" | "needs_login" | "analyzing" | "ready" | "failed";
  loginStatus: "unknown" | "needs_login" | "verified";
  analysisStatus: "not_started" | "running" | "ready" | "failed";
  accountLabel?: string | null;
  path: string;
  files: string[];
}

export interface ProjectState {
  config: ProjectConfig;
  agentProvider: AgentProviderStatus;
  docs: ContextDoc[];
  channelSetups: ChannelSetup[];
  latestRun?: RunState | null;
  runActivity: RunActivity[];
}
