use chrono::{Datelike, Duration, Local, NaiveTime, TimeZone, Utc, Weekday};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::{thread, time::Duration as StdDuration};
use tauri::{Emitter, Manager, State};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
enum AppError {
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("clipboard error: {0}")]
    Clipboard(String),
    #[error("open URL error: {0}")]
    Open(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

type AppResult<T> = Result<T, AppError>;

#[derive(Clone, Default)]
struct SchedulerState {
    active_projects: Arc<Mutex<HashSet<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateProjectRequest {
    project_name: String,
    website_url: String,
    repo_url: Option<String>,
    project_path: String,
    start_analysis: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecentProject {
    id: String,
    name: String,
    path: String,
    website_url: String,
    repo_url: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectConfig {
    id: String,
    name: String,
    website_url: String,
    repo_url: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectStateDto {
    config: ProjectConfig,
    path: String,
    codex_available: bool,
    docs: Vec<DocDto>,
    schedules: Vec<ScheduleManifest>,
    events: Vec<EventDto>,
    drafts: Vec<DraftDto>,
    runs: Vec<RunDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocDto {
    key: String,
    file_name: String,
    title: String,
    content: String,
    updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveDocRequest {
    project_path: String,
    key: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunTaskRequest {
    project_path: String,
    task_kind: String,
    schedule_id: Option<String>,
    custom_instruction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunDto {
    id: String,
    task_kind: String,
    schedule_id: Option<String>,
    status: String,
    started_at: String,
    completed_at: Option<String>,
    codex_session_id: Option<String>,
    summary: Option<String>,
    log_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventDto {
    id: String,
    event_type: String,
    task_id: Option<String>,
    task_kind: Option<String>,
    summary: String,
    payload: Value,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DraftDto {
    id: String,
    channel: String,
    source_url: Option<String>,
    title: String,
    body: String,
    status: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleManifest {
    id: String,
    name: String,
    task_kind: String,
    enabled: bool,
    cadence: Cadence,
    time_of_day: String,
    day_of_week: Option<u32>,
    last_run_at: Option<String>,
    next_run_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Cadence {
    Daily,
    Weekly,
    ThreeTimesWeekly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateScheduleRequest {
    project_path: String,
    schedule: ScheduleManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DraftActionRequest {
    project_path: String,
    draft_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveDraftRequest {
    project_path: String,
    draft_id: String,
    body: String,
}

const DOCS: [(&str, &str, &str); 4] = [
    (
        "product_information",
        "product-information.md",
        "Product Information",
    ),
    (
        "marketing_strategy",
        "marketing-strategy.md",
        "Marketing Strategy",
    ),
    (
        "competitor_analysis",
        "competitor-analysis.md",
        "Competitor Analysis",
    ),
    ("brand_voice", "brand-voice.md", "Brand Voice"),
];

pub fn run() {
    tauri::Builder::default()
        .manage(SchedulerState::default())
        .invoke_handler(tauri::generate_handler![
            default_project_path,
            list_recent_projects,
            create_project,
            load_project,
            save_doc,
            run_task,
            update_schedule,
            set_schedule_enabled,
            start_scheduler,
            approve_draft,
            reject_draft,
            save_draft,
            open_project_in_codex
        ])
        .run(tauri::generate_context!())
        .expect("error while running GTM Agent");
}

#[tauri::command]
fn default_project_path(project_name: String) -> Result<String, AppError> {
    let home =
        dirs::home_dir().ok_or_else(|| AppError::Invalid("cannot locate home directory".into()))?;
    let slug = slugify(&project_name);
    Ok(home
        .join("GTM Agent Projects")
        .join(if slug.is_empty() {
            "new-project"
        } else {
            &slug
        })
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
fn list_recent_projects(app: tauri::AppHandle) -> Result<Vec<RecentProject>, AppError> {
    read_recent_projects(&app)
}

#[tauri::command]
fn create_project(
    app: tauri::AppHandle,
    request: CreateProjectRequest,
) -> Result<ProjectStateDto, AppError> {
    let inferred_name = if request.project_name.trim().is_empty() {
        infer_project_name(&request.website_url)
    } else {
        request.project_name.trim().to_string()
    };
    let requested_path = request.project_path.trim();
    let project_path = if requested_path.is_empty() {
        dirs::home_dir()
            .ok_or_else(|| AppError::Invalid("cannot locate home directory".into()))?
            .join("GTM Agent Projects")
            .join(slugify(&inferred_name))
    } else {
        PathBuf::from(requested_path)
    };
    if !request.website_url.starts_with("http://") && !request.website_url.starts_with("https://") {
        return Err(AppError::Invalid(
            "website URL must start with http:// or https://".into(),
        ));
    }
    fs::create_dir_all(project_path.join("automations"))?;
    fs::create_dir_all(project_path.join("skills/reddit-outreach"))?;
    fs::create_dir_all(project_path.join("skills/competitor-monitor"))?;
    fs::create_dir_all(project_path.join(".gtm-agent/drafts"))?;
    fs::create_dir_all(project_path.join(".gtm-agent/runs"))?;
    fs::create_dir_all(project_path.join(".gtm-agent/bin"))?;

    let now = Utc::now().to_rfc3339();
    let config = ProjectConfig {
        id: format!("project_{}", Uuid::new_v4().simple()),
        name: inferred_name,
        website_url: request.website_url.trim().to_string(),
        repo_url: request.repo_url.as_ref().and_then(|v| {
            let t = v.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }),
        created_at: now.clone(),
        updated_at: now,
    };
    write_json_pretty(&project_path.join(".gtm-agent/config.json"), &config)?;
    write_project_templates(&project_path, &config)?;
    init_db(&project_path)?;
    write_default_schedules(&project_path)?;
    append_event(
        &project_path,
        "project.created",
        None,
        None,
        format!("Created local GTM workspace for {}", config.name),
        json!({ "websiteUrl": config.website_url, "repoUrl": config.repo_url }),
    )?;
    upsert_recent_project(&app, &project_path, &config)?;
    if request.start_analysis {
        spawn_codex_run(&app, &project_path, "initial_analysis", None, None)?;
    }
    load_project(app, project_path.to_string_lossy().to_string())
}

#[tauri::command]
fn load_project(app: tauri::AppHandle, project_path: String) -> Result<ProjectStateDto, AppError> {
    let path = PathBuf::from(project_path);
    let config: ProjectConfig = read_json(&path.join(".gtm-agent/config.json"))?;
    init_db(&path)?;
    ingest_events(&path)?;
    upsert_recent_project(&app, &path, &config)?;
    Ok(ProjectStateDto {
        config,
        path: path.to_string_lossy().to_string(),
        codex_available: codex_available(),
        docs: read_docs(&path)?,
        schedules: read_schedules(&path)?,
        events: read_events(&path)?,
        drafts: read_drafts(&path)?,
        runs: read_runs(&path)?,
    })
}

#[tauri::command]
fn save_doc(request: SaveDocRequest) -> Result<DocDto, AppError> {
    let path = PathBuf::from(request.project_path);
    let (_, file_name, title) = DOCS
        .iter()
        .find(|(key, _, _)| key == &request.key)
        .ok_or_else(|| AppError::Invalid("unknown document key".into()))?;
    fs::write(path.join(file_name), request.content.as_bytes())?;
    append_event(
        &path,
        "context.updated",
        None,
        None,
        format!("Updated {}", file_name),
        json!({ "doc": request.key }),
    )?;
    Ok(DocDto {
        key: request.key,
        file_name: (*file_name).to_string(),
        title: (*title).to_string(),
        content: fs::read_to_string(path.join(file_name))?,
        updated_at: file_updated_at(&path.join(file_name)),
    })
}

#[tauri::command]
fn run_task(app: tauri::AppHandle, request: RunTaskRequest) -> Result<RunDto, AppError> {
    let project_path = PathBuf::from(request.project_path);
    let run_id = spawn_codex_run(
        &app,
        &project_path,
        &request.task_kind,
        request.schedule_id.as_deref(),
        request.custom_instruction.as_deref(),
    )?;
    read_run(&project_path, &run_id)
}

#[tauri::command]
fn update_schedule(request: UpdateScheduleRequest) -> Result<ScheduleManifest, AppError> {
    let project_path = PathBuf::from(request.project_path);
    let mut schedule = request.schedule;
    schedule.next_run_at = compute_next_run(&schedule, Local::now()).map(|dt| dt.to_rfc3339());
    write_schedule(&project_path, &schedule)?;
    append_event(
        &project_path,
        "schedule.updated",
        None,
        Some(schedule.task_kind.clone()),
        format!("Updated schedule {}", schedule.name),
        json!({ "scheduleId": schedule.id }),
    )?;
    Ok(schedule)
}

#[tauri::command]
fn set_schedule_enabled(
    project_path: String,
    schedule_id: String,
    enabled: bool,
) -> Result<ScheduleManifest, AppError> {
    let path = PathBuf::from(project_path);
    let mut schedule = read_schedule(&path, &schedule_id)?;
    schedule.enabled = enabled;
    schedule.next_run_at = compute_next_run(&schedule, Local::now()).map(|dt| dt.to_rfc3339());
    write_schedule(&path, &schedule)?;
    append_event(
        &path,
        if enabled {
            "schedule.enabled"
        } else {
            "schedule.paused"
        },
        None,
        Some(schedule.task_kind.clone()),
        format!(
            "{} {}",
            if enabled { "Enabled" } else { "Paused" },
            schedule.name
        ),
        json!({ "scheduleId": schedule.id }),
    )?;
    Ok(schedule)
}

#[tauri::command]
fn start_scheduler(
    app: tauri::AppHandle,
    state: State<SchedulerState>,
    project_path: String,
) -> Result<(), AppError> {
    let canonical = PathBuf::from(&project_path)
        .canonicalize()?
        .to_string_lossy()
        .to_string();
    {
        let mut active = state
            .active_projects
            .lock()
            .map_err(|_| AppError::Invalid("scheduler lock poisoned".into()))?;
        if active.contains(&canonical) {
            return Ok(());
        }
        active.insert(canonical.clone());
    }
    thread::spawn(move || loop {
        let project = PathBuf::from(&canonical);
        if let Err(err) = run_due_schedules(&app, &project) {
            let _ = append_event(
                &project,
                "scheduler.error",
                None,
                None,
                format!("Scheduler error: {}", err),
                json!({ "error": err.to_string() }),
            );
        }
        thread::sleep(StdDuration::from_secs(30));
    });
    Ok(())
}

#[tauri::command]
fn approve_draft(request: DraftActionRequest) -> Result<DraftDto, AppError> {
    let path = PathBuf::from(request.project_path);
    let mut draft = read_draft(&path, &request.draft_id)?;
    arboard::Clipboard::new()
        .map_err(|err| AppError::Clipboard(err.to_string()))?
        .set_text(draft.body.clone())
        .map_err(|err| AppError::Clipboard(err.to_string()))?;
    if let Some(url) = &draft.source_url {
        opener::open(url).map_err(|err| AppError::Open(err.to_string()))?;
    }
    draft.status = "approved".into();
    draft.updated_at = Utc::now().to_rfc3339();
    write_draft(&path, &draft)?;
    append_event(
        &path,
        "draft.approved",
        None,
        Some(draft.channel.clone()),
        format!("Approved draft {}", draft.title),
        json!({ "draftId": draft.id, "sourceUrl": draft.source_url }),
    )?;
    Ok(draft)
}

#[tauri::command]
fn reject_draft(request: DraftActionRequest) -> Result<DraftDto, AppError> {
    let path = PathBuf::from(request.project_path);
    let mut draft = read_draft(&path, &request.draft_id)?;
    draft.status = "rejected".into();
    draft.updated_at = Utc::now().to_rfc3339();
    write_draft(&path, &draft)?;
    append_event(
        &path,
        "draft.rejected",
        None,
        Some(draft.channel.clone()),
        format!("Rejected draft {}", draft.title),
        json!({ "draftId": draft.id }),
    )?;
    Ok(draft)
}

#[tauri::command]
fn save_draft(request: SaveDraftRequest) -> Result<DraftDto, AppError> {
    let path = PathBuf::from(request.project_path);
    let mut draft = read_draft(&path, &request.draft_id)?;
    draft.body = request.body;
    draft.updated_at = Utc::now().to_rfc3339();
    write_draft(&path, &draft)?;
    append_event(
        &path,
        "draft.updated",
        None,
        Some(draft.channel.clone()),
        format!("Updated draft {}", draft.title),
        json!({ "draftId": draft.id }),
    )?;
    Ok(draft)
}

#[tauri::command]
fn open_project_in_codex(project_path: String) -> Result<(), AppError> {
    let binary = codex_binary().ok_or_else(|| AppError::Open("codex binary not found".into()))?;
    Command::new(binary)
        .arg("app")
        .arg(project_path)
        .spawn()
        .map_err(|err| AppError::Open(err.to_string()))?;
    Ok(())
}

fn write_project_templates(project_path: &Path, config: &ProjectConfig) -> AppResult<()> {
    fs::write(
        project_path.join("AGENTS.md"),
        format!(
            "# GTM Agent Project: {}\n\nThis is a local Codex workspace managed by GTM Agent. Codex does the GTM work; the desktop app manages visibility, schedules, and review.\n\n## Source of truth\n\n- `product-information.md`: what the product is, what it does, and proof points.\n- `marketing-strategy.md`: ICP, personas, channels, positioning, and campaign angles.\n- `competitor-analysis.md`: competitor set, alternatives, positioning gaps, and changes.\n- `brand-voice.md`: voice, tone, vocabulary, messages, and public-reply rules.\n\n## Event protocol\n\nWhen performing GTM tasks, append JSONL events to `.gtm-agent/events.jsonl` with fields `id`, `eventType`, `taskId`, `taskKind`, `summary`, `payload`, and `createdAt`.\n\nDrafts must be written as JSON files under `.gtm-agent/drafts/` with fields `id`, `channel`, `sourceUrl`, `title`, `body`, `status`, `createdAt`, and `updatedAt`.\n\n## Project inputs\n\nWebsite: {}\nRepo: {}\n",
            config.name,
            config.website_url,
            config.repo_url.as_deref().unwrap_or("not provided")
        ),
    )?;
    fs::write(
        project_path.join("product-information.md"),
        format!(
            "# Product Information\n\n_Status: pending Codex analysis_\n\n## Product\n\nWebsite: {}\n\n## What it does\n\nPending.\n\n## Core features\n\nPending.\n\n## Proof points\n\nPending.\n",
            config.website_url
        ),
    )?;
    fs::write(
        project_path.join("marketing-strategy.md"),
        "# Marketing Strategy\n\n_Status: pending Codex analysis_\n\n## ICP\n\nPending.\n\n## Personas\n\nPending.\n\n## Pain points\n\nPending.\n\n## Channels\n\nPending.\n\n## Campaign angles\n\nPending.\n",
    )?;
    fs::write(
        project_path.join("competitor-analysis.md"),
        "# Competitor Analysis\n\n_Status: pending Codex analysis_\n\n## Direct competitors\n\nPending.\n\n## Alternatives\n\nPending.\n\n## Positioning gaps\n\nPending.\n",
    )?;
    fs::write(
        project_path.join("brand-voice.md"),
        "# Brand Voice\n\n_Status: pending Codex analysis_\n\n## Voice\n\nPending.\n\n## Tone rules\n\nPending.\n\n## Vocabulary\n\nPending.\n\n## Key messages\n\nPending.\n",
    )?;
    fs::write(
        project_path.join("skills/reddit-outreach/SKILL.md"),
        "# Reddit Outreach Skill\n\nRead `product-information.md`, `marketing-strategy.md`, `competitor-analysis.md`, and `brand-voice.md` first. Search current public Reddit discussions relevant to the ICP and product category. Do not post. Create draft JSON files in `.gtm-agent/drafts/` and append structured events to `.gtm-agent/events.jsonl`.\n",
    )?;
    fs::write(
        project_path.join("skills/competitor-monitor/SKILL.md"),
        "# Competitor Monitor Skill\n\nRead `product-information.md`, `marketing-strategy.md`, `competitor-analysis.md`, and `brand-voice.md` first. Visit competitor sites or current public sources where useful. Append structured events to `.gtm-agent/events.jsonl` and update `competitor-analysis.md` only when there is concrete evidence.\n",
    )?;
    fs::write(
        project_path.join(".gtm-agent/bin/gtm-event.mjs"),
        event_helper(),
    )?;
    Ok(())
}

fn event_helper() -> &'static str {
    r##"#!/usr/bin/env node
import fs from 'node:fs';
import path from 'node:path';
import { randomUUID } from 'node:crypto';
const [, , eventType, summary = '', payloadRaw = '{}'] = process.argv;
if (!eventType) {
  console.error('usage: gtm-event <eventType> <summary> [payloadJson]');
  process.exit(1);
}
const root = process.cwd();
const eventPath = path.join(root, '.gtm-agent', 'events.jsonl');
let payload = {};
try { payload = JSON.parse(payloadRaw); } catch { payload = { raw: payloadRaw }; }
const event = {
  id: `event_${randomUUID()}`,
  eventType,
  taskId: process.env.GTM_TASK_ID || null,
  taskKind: process.env.GTM_TASK_KIND || null,
  summary,
  payload,
  createdAt: new Date().toISOString()
};
fs.mkdirSync(path.dirname(eventPath), { recursive: true });
fs.appendFileSync(eventPath, JSON.stringify(event) + '\n');
"##
}

fn write_default_schedules(project_path: &Path) -> AppResult<()> {
    let schedules = vec![
        ScheduleManifest {
            id: "reddit-daily".into(),
            name: "Reddit outreach scan".into(),
            task_kind: "reddit_outreach".into(),
            enabled: false,
            cadence: Cadence::Daily,
            time_of_day: "09:00".into(),
            day_of_week: None,
            last_run_at: None,
            next_run_at: None,
        },
        ScheduleManifest {
            id: "competitors-weekly".into(),
            name: "Competitor monitor".into(),
            task_kind: "competitor_monitor".into(),
            enabled: false,
            cadence: Cadence::Weekly,
            time_of_day: "10:00".into(),
            day_of_week: Some(1),
            last_run_at: None,
            next_run_at: None,
        },
    ];
    for mut schedule in schedules {
        schedule.next_run_at = compute_next_run(&schedule, Local::now()).map(|dt| dt.to_rfc3339());
        write_schedule(project_path, &schedule)?;
    }
    Ok(())
}

fn spawn_codex_run(
    app: &tauri::AppHandle,
    project_path: &Path,
    task_kind: &str,
    schedule_id: Option<&str>,
    custom_instruction: Option<&str>,
) -> AppResult<String> {
    init_db(project_path)?;
    let run_id = format!("run_{}", Utc::now().format("%Y%m%d%H%M%S"));
    let log_path = project_path
        .join(".gtm-agent/runs")
        .join(format!("{}.jsonl", run_id));
    let now = Utc::now().to_rfc3339();
    let run = RunDto {
        id: run_id.clone(),
        task_kind: task_kind.into(),
        schedule_id: schedule_id.map(str::to_string),
        status: "running".into(),
        started_at: now.clone(),
        completed_at: None,
        codex_session_id: None,
        summary: Some("Codex run started".into()),
        log_path: log_path.to_string_lossy().to_string(),
    };
    upsert_run(project_path, &run)?;
    append_event(
        project_path,
        "task.started",
        Some(run_id.clone()),
        Some(task_kind.to_string()),
        format!("Started {}", human_task(task_kind)),
        json!({ "scheduleId": schedule_id }),
    )?;

    let prompt = build_prompt(
        project_path,
        task_kind,
        &run_id,
        schedule_id,
        custom_instruction,
    )?;
    let app_handle = app.clone();
    let run_id_for_thread = run_id.clone();
    let project = project_path.to_path_buf();
    let task = task_kind.to_string();
    let schedule = schedule_id.map(str::to_string);
    thread::spawn(move || {
        let result = execute_codex(&project, &run_id_for_thread, &task, &prompt, &log_path);
        let completed = Utc::now().to_rfc3339();
        let (status, summary) = match result {
            Ok(()) => (
                "completed".to_string(),
                format!("Completed {}", human_task(&task)),
            ),
            Err(err) => (
                "failed".to_string(),
                format!("{} failed: {}", human_task(&task), err),
            ),
        };
        let finished = RunDto {
            id: run_id_for_thread.clone(),
            task_kind: task.clone(),
            schedule_id: schedule.clone(),
            status: status.clone(),
            started_at: now,
            completed_at: Some(completed),
            codex_session_id: None,
            summary: Some(summary.clone()),
            log_path: log_path.to_string_lossy().to_string(),
        };
        let _ = upsert_run(&project, &finished);
        let _ = append_event(
            &project,
            if status == "completed" {
                "task.completed"
            } else {
                "task.failed"
            },
            Some(run_id_for_thread.clone()),
            Some(task.clone()),
            summary,
            json!({ "status": status, "scheduleId": schedule }),
        );
        let _ = ingest_events(&project);
        let _ = app_handle.emit(
            "project-updated",
            json!({ "projectPath": project.to_string_lossy(), "runId": run_id_for_thread }),
        );
    });
    Ok(run_id)
}

fn execute_codex(
    project: &Path,
    run_id: &str,
    task_kind: &str,
    prompt: &str,
    log_path: &Path,
) -> AppResult<()> {
    let binary = codex_binary().ok_or_else(|| {
        AppError::Invalid(
            "codex binary not found. Install Codex or set CODEX_BIN to the codex executable path"
                .into(),
        )
    })?;
    let mut child = Command::new(binary)
        .arg("--search")
        .arg("exec")
        .arg("--json")
        .arg("--cd")
        .arg(project)
        .arg("--ask-for-approval")
        .arg("never")
        .arg(prompt)
        .env("GTM_PROJECT_PATH", project)
        .env("GTM_TASK_ID", run_id)
        .env("GTM_TASK_KIND", task_kind)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| AppError::Invalid(format!("failed to launch codex: {}", err)))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AppError::Invalid("missing codex stdout".into()))?;
    let mut writer = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    for line in BufReader::new(stdout).lines() {
        let line = line?;
        writeln!(writer, "{}", line)?;
    }
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::Invalid(format!("codex exited with {}", status)))
    }
}

fn build_prompt(
    project_path: &Path,
    task_kind: &str,
    run_id: &str,
    schedule_id: Option<&str>,
    custom_instruction: Option<&str>,
) -> AppResult<String> {
    let config: ProjectConfig = read_json(&project_path.join(".gtm-agent/config.json"))?;
    let task_body = match task_kind {
        "initial_analysis" => format!("Analyze the project deeply. Visit the website {} and inspect the repository if available: {}. Rewrite product-information.md, marketing-strategy.md, competitor-analysis.md, and brand-voice.md with concrete findings. Append progress and completion events to .gtm-agent/events.jsonl.", config.website_url, config.repo_url.as_deref().unwrap_or("not provided")),
        "context_review" => "Review and improve the context markdown files. Preserve user edits that are more specific than your inference. Append events describing what changed.".into(),
        "reddit_outreach" => "Run the project-local Reddit outreach workflow. Read context docs, find relevant current Reddit opportunities, create JSON draft files under .gtm-agent/drafts/, and append opportunity/draft events. Do not post anywhere.".into(),
        "competitor_monitor" => "Run the project-local competitor monitoring workflow. Read competitor-analysis.md and current public sources, update competitor-analysis.md only with evidenced changes, and append events.".into(),
        other => custom_instruction.unwrap_or(other).to_string(),
    };
    Ok(format!("# GTM Agent Task: {}\nProject: {}\nTask ID: {}\nSchedule ID: {}\nWorkspace: {}\n\nYou are running inside a local Codex project workspace managed by GTM Agent. Use the filesystem as the source of truth. Do not call any external LLM API. Do not post publicly or send messages.\n\nEvent schema: append JSON lines to .gtm-agent/events.jsonl with id, eventType, taskId, taskKind, summary, payload, createdAt. Draft schema: write .gtm-agent/drafts/<draft_id>.json with id, channel, sourceUrl, title, body, status, createdAt, updatedAt.\n\n{}\n\nAdditional instruction: {}\n", human_task(task_kind), config.name, run_id, schedule_id.unwrap_or("manual"), project_path.to_string_lossy(), task_body, custom_instruction.unwrap_or("none")))
}

fn run_due_schedules(app: &tauri::AppHandle, project: &Path) -> AppResult<()> {
    let now = Local::now();
    for mut schedule in read_schedules(project)? {
        if !schedule.enabled {
            continue;
        }
        let due = schedule
            .next_run_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Local) <= now)
            .unwrap_or(true);
        if due {
            spawn_codex_run(app, project, &schedule.task_kind, Some(&schedule.id), None)?;
            schedule.last_run_at = Some(Utc::now().to_rfc3339());
            schedule.next_run_at =
                compute_next_run(&schedule, now + Duration::minutes(1)).map(|dt| dt.to_rfc3339());
            write_schedule(project, &schedule)?;
        }
    }
    Ok(())
}

fn init_db(project_path: &Path) -> AppResult<()> {
    fs::create_dir_all(project_path.join(".gtm-agent"))?;
    let conn = Connection::open(project_path.join(".gtm-agent/gtm.sqlite"))?;
    conn.execute_batch("CREATE TABLE IF NOT EXISTS events (id TEXT PRIMARY KEY, event_type TEXT NOT NULL, task_id TEXT, task_kind TEXT, summary TEXT NOT NULL, payload TEXT NOT NULL, created_at TEXT NOT NULL); CREATE TABLE IF NOT EXISTS runs (id TEXT PRIMARY KEY, task_kind TEXT NOT NULL, schedule_id TEXT, status TEXT NOT NULL, started_at TEXT NOT NULL, completed_at TEXT, codex_session_id TEXT, summary TEXT, log_path TEXT NOT NULL);")?;
    Ok(())
}

fn append_event(
    project_path: &Path,
    event_type: &str,
    task_id: Option<String>,
    task_kind: Option<String>,
    summary: String,
    payload: Value,
) -> AppResult<()> {
    fs::create_dir_all(project_path.join(".gtm-agent"))?;
    let event = EventDto {
        id: format!("event_{}", Uuid::new_v4().simple()),
        event_type: event_type.into(),
        task_id,
        task_kind,
        summary,
        payload,
        created_at: Utc::now().to_rfc3339(),
    };
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(project_path.join(".gtm-agent/events.jsonl"))?;
    writeln!(file, "{}", serde_json::to_string(&event)?)?;
    insert_event(project_path, &event)?;
    Ok(())
}

fn ingest_events(project_path: &Path) -> AppResult<()> {
    let path = project_path.join(".gtm-agent/events.jsonl");
    if !path.exists() {
        return Ok(());
    }
    for line in fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.trim().is_empty())
    {
        let value: Value = serde_json::from_str(line)?;
        let event = EventDto {
            id: string_field(&value, &["id"])
                .unwrap_or_else(|| format!("event_{}", Uuid::new_v4().simple())),
            event_type: string_field(&value, &["eventType", "event_type"])
                .unwrap_or_else(|| "event".into()),
            task_id: string_field(&value, &["taskId", "task_id"]),
            task_kind: string_field(&value, &["taskKind", "task_kind"]),
            summary: string_field(&value, &["summary"]).unwrap_or_else(|| "Event".into()),
            payload: value.get("payload").cloned().unwrap_or_else(|| json!({})),
            created_at: string_field(&value, &["createdAt", "created_at"])
                .unwrap_or_else(|| Utc::now().to_rfc3339()),
        };
        insert_event(project_path, &event)?;
    }
    Ok(())
}

fn insert_event(project_path: &Path, event: &EventDto) -> AppResult<()> {
    init_db(project_path)?;
    let conn = Connection::open(project_path.join(".gtm-agent/gtm.sqlite"))?;
    conn.execute("INSERT OR IGNORE INTO events (id,event_type,task_id,task_kind,summary,payload,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7)", params![event.id, event.event_type, event.task_id, event.task_kind, event.summary, event.payload.to_string(), event.created_at])?;
    Ok(())
}

fn read_events(project_path: &Path) -> AppResult<Vec<EventDto>> {
    let conn = Connection::open(project_path.join(".gtm-agent/gtm.sqlite"))?;
    let mut stmt = conn.prepare("SELECT id,event_type,task_id,task_kind,summary,payload,created_at FROM events ORDER BY created_at DESC LIMIT 200")?;
    let rows = stmt.query_map([], |row| {
        let payload: String = row.get(5)?;
        Ok(EventDto {
            id: row.get(0)?,
            event_type: row.get(1)?,
            task_id: row.get(2)?,
            task_kind: row.get(3)?,
            summary: row.get(4)?,
            payload: serde_json::from_str(&payload).unwrap_or_else(|_| json!({})),
            created_at: row.get(6)?,
        })
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn upsert_run(project_path: &Path, run: &RunDto) -> AppResult<()> {
    init_db(project_path)?;
    let conn = Connection::open(project_path.join(".gtm-agent/gtm.sqlite"))?;
    conn.execute("INSERT OR REPLACE INTO runs (id,task_kind,schedule_id,status,started_at,completed_at,codex_session_id,summary,log_path) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)", params![run.id, run.task_kind, run.schedule_id, run.status, run.started_at, run.completed_at, run.codex_session_id, run.summary, run.log_path])?;
    Ok(())
}

fn read_run(project_path: &Path, run_id: &str) -> AppResult<RunDto> {
    read_runs(project_path)?
        .into_iter()
        .find(|run| run.id == run_id)
        .ok_or_else(|| AppError::Invalid("run not found".into()))
}

fn read_runs(project_path: &Path) -> AppResult<Vec<RunDto>> {
    let conn = Connection::open(project_path.join(".gtm-agent/gtm.sqlite"))?;
    let mut stmt = conn.prepare("SELECT id,task_kind,schedule_id,status,started_at,completed_at,codex_session_id,summary,log_path FROM runs ORDER BY started_at DESC LIMIT 100")?;
    let rows = stmt.query_map([], |row| {
        Ok(RunDto {
            id: row.get(0)?,
            task_kind: row.get(1)?,
            schedule_id: row.get(2)?,
            status: row.get(3)?,
            started_at: row.get(4)?,
            completed_at: row.get(5)?,
            codex_session_id: row.get(6)?,
            summary: row.get(7)?,
            log_path: row.get(8)?,
        })
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn read_docs(project_path: &Path) -> AppResult<Vec<DocDto>> {
    DOCS.iter()
        .map(|(key, file, title)| {
            let path = project_path.join(file);
            Ok(DocDto {
                key: (*key).into(),
                file_name: (*file).into(),
                title: (*title).into(),
                content: fs::read_to_string(&path).unwrap_or_default(),
                updated_at: file_updated_at(&path),
            })
        })
        .collect()
}

fn read_schedules(project_path: &Path) -> AppResult<Vec<ScheduleManifest>> {
    let dir = project_path.join("automations");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut schedules = vec![];
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|v| v.to_str()) == Some("json") {
            schedules.push(read_json(&path)?);
        }
    }
    schedules.sort_by(|a: &ScheduleManifest, b| a.name.cmp(&b.name));
    Ok(schedules)
}

fn read_schedule(project_path: &Path, id: &str) -> AppResult<ScheduleManifest> {
    read_json(
        &project_path
            .join("automations")
            .join(format!("{}.json", id)),
    )
}
fn write_schedule(project_path: &Path, schedule: &ScheduleManifest) -> AppResult<()> {
    write_json_pretty(
        &project_path
            .join("automations")
            .join(format!("{}.json", schedule.id)),
        schedule,
    )
}

fn read_drafts(project_path: &Path) -> AppResult<Vec<DraftDto>> {
    let dir = project_path.join(".gtm-agent/drafts");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut drafts = vec![];
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|v| v.to_str()) == Some("json") {
            drafts.push(read_json(&path)?);
        }
    }
    drafts.sort_by(|a: &DraftDto, b| b.created_at.cmp(&a.created_at));
    Ok(drafts)
}
fn read_draft(project_path: &Path, id: &str) -> AppResult<DraftDto> {
    read_json(
        &project_path
            .join(".gtm-agent/drafts")
            .join(format!("{}.json", id)),
    )
}
fn write_draft(project_path: &Path, draft: &DraftDto) -> AppResult<()> {
    write_json_pretty(
        &project_path
            .join(".gtm-agent/drafts")
            .join(format!("{}.json", draft.id)),
        draft,
    )
}

fn read_recent_projects(app: &tauri::AppHandle) -> AppResult<Vec<RecentProject>> {
    let path = recent_path(app)?;
    if !path.exists() {
        return Ok(vec![]);
    }
    read_json(&path)
}

fn upsert_recent_project(
    app: &tauri::AppHandle,
    path: &Path,
    config: &ProjectConfig,
) -> AppResult<()> {
    let mut projects = read_recent_projects(app).unwrap_or_default();
    projects.retain(|p| p.id != config.id && p.path != path.to_string_lossy());
    projects.insert(
        0,
        RecentProject {
            id: config.id.clone(),
            name: config.name.clone(),
            path: path.to_string_lossy().to_string(),
            website_url: config.website_url.clone(),
            repo_url: config.repo_url.clone(),
            created_at: config.created_at.clone(),
            updated_at: Utc::now().to_rfc3339(),
        },
    );
    projects.truncate(20);
    let recent = recent_path(app)?;
    fs::create_dir_all(recent.parent().unwrap())?;
    write_json_pretty(&recent, &projects)
}

fn recent_path(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|err| AppError::Invalid(err.to_string()))?;
    Ok(dir.join("recent-projects.json"))
}

fn compute_next_run(
    schedule: &ScheduleManifest,
    now: chrono::DateTime<Local>,
) -> Option<chrono::DateTime<Local>> {
    if !schedule.enabled {
        return None;
    }
    let time = NaiveTime::parse_from_str(&schedule.time_of_day, "%H:%M").ok()?;
    for offset in 0..14 {
        let date = now.date_naive() + Duration::days(offset);
        let candidate = Local.from_local_datetime(&date.and_time(time)).single()?;
        if candidate <= now {
            continue;
        }
        match schedule.cadence {
            Cadence::Daily => return Some(candidate),
            Cadence::Weekly => {
                let day = schedule.day_of_week.unwrap_or(1);
                if weekday_number(candidate.weekday()) == day {
                    return Some(candidate);
                }
            }
            Cadence::ThreeTimesWeekly => {
                if matches!(
                    candidate.weekday(),
                    Weekday::Mon | Weekday::Wed | Weekday::Fri
                ) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn weekday_number(day: Weekday) -> u32 {
    match day {
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
        Weekday::Sun => 7,
    }
}
fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> AppResult<T> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}
fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?.as_bytes())?;
    Ok(())
}
fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(|v| v.as_str()).map(str::to_string))
}
fn file_updated_at(path: &Path) -> Option<String> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()
        .map(chrono::DateTime::<Utc>::from)
        .map(|dt| dt.to_rfc3339())
}
fn codex_available() -> bool {
    codex_binary()
        .and_then(|binary| {
            Command::new(binary)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .ok()
        })
        .map(|status| status.success())
        .unwrap_or(false)
}
fn codex_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("CODEX_BIN") {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    for path in [
        "/opt/homebrew/bin/codex",
        "/usr/local/bin/codex",
        "/usr/bin/codex",
    ] {
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    if let Ok(output) = Command::new("sh")
        .arg("-lc")
        .arg("command -v codex")
        .output()
    {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

fn infer_project_name(website_url: &str) -> String {
    let without_scheme = website_url
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = without_scheme.split('/').next().unwrap_or("product");
    let host = host.strip_prefix("www.").unwrap_or(host);
    let label = host.split('.').next().unwrap_or(host);
    if label.is_empty() {
        "Product".into()
    } else {
        let mut chars = label.chars();
        match chars.next() {
            Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
            None => "Product".into(),
        }
    }
}

fn slugify(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
fn human_task(task: &str) -> String {
    task.replace('_', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slugify_project_names() {
        assert_eq!(slugify("Acme GTM Agent!"), "acme-gtm-agent");
    }
    #[test]
    fn disabled_schedule_has_no_next_run() {
        let schedule = ScheduleManifest {
            id: "x".into(),
            name: "x".into(),
            task_kind: "reddit_outreach".into(),
            enabled: false,
            cadence: Cadence::Daily,
            time_of_day: "09:00".into(),
            day_of_week: None,
            last_run_at: None,
            next_run_at: None,
        };
        assert!(compute_next_run(&schedule, Local::now()).is_none());
    }
    #[test]
    fn enabled_daily_schedule_gets_next_run() {
        let schedule = ScheduleManifest {
            id: "x".into(),
            name: "x".into(),
            task_kind: "reddit_outreach".into(),
            enabled: true,
            cadence: Cadence::Daily,
            time_of_day: "09:00".into(),
            day_of_week: None,
            last_run_at: None,
            next_run_at: None,
        };
        assert!(compute_next_run(&schedule, Local::now()).is_some());
    }
}
