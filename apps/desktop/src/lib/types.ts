export type Cadence = "daily" | "weekly" | "three_times_weekly";

export interface RecentProject {
  id: string;
  name: string;
  path: string;
  websiteUrl: string;
  repoUrl?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ProjectConfig {
  id: string;
  name: string;
  websiteUrl: string;
  repoUrl?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface DocDto {
  key: string;
  fileName: string;
  title: string;
  content: string;
  updatedAt?: string | null;
}

export interface ScheduleManifest {
  id: string;
  name: string;
  taskKind: string;
  enabled: boolean;
  cadence: Cadence;
  timeOfDay: string;
  dayOfWeek?: number | null;
  lastRunAt?: string | null;
  nextRunAt?: string | null;
}

export interface EventDto {
  id: string;
  eventType: string;
  taskId?: string | null;
  taskKind?: string | null;
  summary: string;
  payload: unknown;
  createdAt: string;
}

export interface DraftDto {
  id: string;
  channel: string;
  sourceUrl?: string | null;
  title: string;
  body: string;
  status: string;
  createdAt: string;
  updatedAt: string;
}

export interface RunDto {
  id: string;
  taskKind: string;
  scheduleId?: string | null;
  status: string;
  startedAt: string;
  completedAt?: string | null;
  codexSessionId?: string | null;
  summary?: string | null;
  logPath: string;
}

export interface ProjectStateDto {
  config: ProjectConfig;
  path: string;
  codexAvailable: boolean;
  docs: DocDto[];
  schedules: ScheduleManifest[];
  events: EventDto[];
  drafts: DraftDto[];
  runs: RunDto[];
}

export type ViewKey = "overview" | "context" | "drafts" | "automations" | "events" | "settings";
