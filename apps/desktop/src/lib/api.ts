import { invoke } from "@tauri-apps/api/core";
import type { DraftDto, ProjectStateDto, RecentProject, ScheduleManifest } from "./types";

export const api = {
  defaultProjectPath(projectName: string) {
    return invoke<string>("default_project_path", { projectName });
  },
  listRecentProjects() {
    return invoke<RecentProject[]>("list_recent_projects");
  },
  createProject(input: {
    projectName: string;
    websiteUrl: string;
    repoUrl?: string;
    projectPath: string;
    startAnalysis: boolean;
  }) {
    return invoke<ProjectStateDto>("create_project", { request: input });
  },
  loadProject(projectPath: string) {
    return invoke<ProjectStateDto>("load_project", { projectPath });
  },
  saveDoc(projectPath: string, key: string, content: string) {
    return invoke("save_doc", { request: { projectPath, key, content } });
  },
  runTask(projectPath: string, taskKind: string, scheduleId?: string, customInstruction?: string) {
    return invoke("run_task", { request: { projectPath, taskKind, scheduleId, customInstruction } });
  },
  updateSchedule(projectPath: string, schedule: ScheduleManifest) {
    return invoke<ScheduleManifest>("update_schedule", { request: { projectPath, schedule } });
  },
  setScheduleEnabled(projectPath: string, scheduleId: string, enabled: boolean) {
    return invoke<ScheduleManifest>("set_schedule_enabled", { projectPath, scheduleId, enabled });
  },
  startScheduler(projectPath: string) {
    return invoke<void>("start_scheduler", { projectPath });
  },
  approveDraft(projectPath: string, draftId: string) {
    return invoke<DraftDto>("approve_draft", { request: { projectPath, draftId } });
  },
  rejectDraft(projectPath: string, draftId: string) {
    return invoke<DraftDto>("reject_draft", { request: { projectPath, draftId } });
  },
  saveDraft(projectPath: string, draftId: string, body: string) {
    return invoke<DraftDto>("save_draft", { request: { projectPath, draftId, body } });
  },
  openProjectInCodex(projectPath: string) {
    return invoke<void>("open_project_in_codex", { projectPath });
  },
};
