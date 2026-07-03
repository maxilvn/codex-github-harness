import React from "react";
import { createRoot } from "react-dom/client";
import ReactMarkdown from "react-markdown";
import { listen } from "@tauri-apps/api/event";
import { api } from "./lib/api";
import { formatDate, label, statusClass } from "./lib/format";
import type { DocDto, DraftDto, ProjectStateDto, RecentProject, ScheduleManifest, ViewKey } from "./lib/types";
import "./styles.css";

const nav: Array<{ key: ViewKey; label: string }> = [
  { key: "overview", label: "Overview" },
  { key: "context", label: "Context" },
  { key: "drafts", label: "Drafts" },
  { key: "automations", label: "Automations" },
  { key: "events", label: "Events" },
  { key: "settings", label: "Settings" },
];

function App() {
  const [project, setProject] = React.useState<ProjectStateDto | null>(null);
  const [recent, setRecent] = React.useState<RecentProject[]>([]);
  const [view, setView] = React.useState<ViewKey>("overview");
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<string | null>(null);

  const refresh = React.useCallback(async () => {
    if (!project) return;
    try {
      setProject(await api.loadProject(project.path));
    } catch (err) {
      setError(String(err));
    }
  }, [project]);

  React.useEffect(() => {
    api.listRecentProjects().then(setRecent).catch(() => setRecent([]));
  }, []);

  React.useEffect(() => {
    if (!project) return;
    api.startScheduler(project.path).catch((err) => setError(String(err)));
    const id = window.setInterval(() => refresh(), 5000);
    return () => window.clearInterval(id);
  }, [project, refresh]);

  React.useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen("project-updated", () => refresh()).then((dispose) => {
      unlisten = dispose;
    }).catch(() => undefined);
    return () => unlisten?.();
  }, [refresh]);

  async function loadProject(path: string) {
    setBusy(true);
    setError(null);
    try {
      const state = await api.loadProject(path);
      setProject(state);
      setView("overview");
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function afterMutation(action: () => Promise<unknown>) {
    setBusy(true);
    setError(null);
    try {
      await action();
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  if (!project) {
    return <Onboarding recent={recent} busy={busy} error={error} onCreate={setProject} onLoad={loadProject} />;
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-mark"><span /> GTM Agent</div>
        <div className="project-switcher">
          <p className="eyebrow">Active project</p>
          <strong>{project.config.name}</strong>
          <code>{project.path}</code>
        </div>
        <nav className="nav-list">
          {nav.map((item) => (
            <button key={item.key} className={view === item.key ? "active" : ""} onClick={() => setView(item.key)}>
              {item.label}
            </button>
          ))}
        </nav>
        <div className="sidebar-footer">
          <span className={project.codexAvailable ? "status success" : "status danger"}>{project.codexAvailable ? "Codex available" : "Codex missing"}</span>
          <button className="button secondary full" onClick={() => api.openProjectInCodex(project.path)}>Open in Codex</button>
        </div>
      </aside>
      <main className="main">
        <header className="topbar">
          <div>
            <p className="eyebrow">{project.config.websiteUrl}</p>
            <h1>{viewTitle(view)}</h1>
          </div>
          <div className="topbar-actions">
            <button className="button secondary" onClick={() => refresh()} disabled={busy}>Refresh</button>
            <button className="button primary" onClick={() => afterMutation(() => api.runTask(project.path, "initial_analysis"))} disabled={busy}>Run analysis</button>
          </div>
        </header>
        {error ? <div className="error-banner">{error}</div> : null}
        {view === "overview" && <Overview project={project} mutate={afterMutation} />}
        {view === "context" && <ContextDocs project={project} mutate={afterMutation} />}
        {view === "drafts" && <Drafts project={project} mutate={afterMutation} />}
        {view === "automations" && <Automations project={project} mutate={afterMutation} />}
        {view === "events" && <Events project={project} />}
        {view === "settings" && <Settings project={project} />}
      </main>
    </div>
  );
}

function Onboarding({ recent, busy, error, onCreate, onLoad }: { recent: RecentProject[]; busy: boolean; error: string | null; onCreate: (project: ProjectStateDto) => void; onLoad: (path: string) => void }) {
  const [brandUrl, setBrandUrl] = React.useState("");
  const [localError, setLocalError] = React.useState<string | null>(null);
  const [creating, setCreating] = React.useState(false);

  function normalizedUrl() {
    const trimmed = brandUrl.trim();
    if (!trimmed) return "";
    return /^https?:\/\//i.test(trimmed) ? trimmed : `https://${trimmed}`;
  }

  function inferredName(url: string) {
    try {
      const host = new URL(url).hostname.replace(/^www\./, "");
      const first = host.split(".")[0] || "product";
      return first.charAt(0).toUpperCase() + first.slice(1);
    } catch {
      return "Product";
    }
  }

  async function create() {
    const websiteUrl = normalizedUrl();
    if (!websiteUrl) {
      setLocalError("Enter a brand URL first.");
      return;
    }
    setCreating(true);
    setLocalError(null);
    try {
      const projectName = inferredName(websiteUrl);
      const projectPath = await api.defaultProjectPath(projectName);
      const project = await api.createProject({ projectName, websiteUrl, repoUrl: "", projectPath, startAnalysis: true });
      onCreate(project);
    } catch (err) {
      setLocalError(String(err));
    } finally {
      setCreating(false);
    }
  }

  return (
    <main className="login-screen">
      <section className="login-card">
        <div className="brand-mark large"><span /> GTM Agent</div>
        <h1>Turn a brand URL into a local GTM agent workspace.</h1>
        <p>Codex analyzes the product, writes the strategy files, and keeps every run inspectable in the project workspace.</p>
        {(error || localError) ? <div className="error-banner">{error || localError}</div> : null}
        <label className="url-field">
          Brand URL
          <input
            autoFocus
            value={brandUrl}
            onChange={(event) => setBrandUrl(event.target.value)}
            onKeyDown={(event) => { if (event.key === "Enter") void create(); }}
            placeholder="taptlk.com"
          />
        </label>
        <button className="button primary login-button" onClick={create} disabled={busy || creating}>{creating ? "Creating workspace…" : "Analyze with Codex"}</button>
        <p className="fine-print">Creates product-information.md, marketing-strategy.md, competitor-analysis.md, and brand-voice.md locally.</p>
      </section>
      {recent.length > 0 ? (
        <section className="recent-strip">
          <p className="eyebrow">Recent</p>
          {recent.slice(0, 3).map((project) => (
            <button className="recent-row" key={project.id} onClick={() => onLoad(project.path)}>
              <strong>{project.name}</strong>
              <span>{project.websiteUrl}</span>
            </button>
          ))}
        </section>
      ) : null}
    </main>
  );
}

function Overview({ project, mutate }: { project: ProjectStateDto; mutate: (action: () => Promise<unknown>) => Promise<void> }) {
  const running = project.runs.filter((run) => run.status === "running");
  const pendingDrafts = project.drafts.filter((draft) => draft.status === "draft");
  return (
    <div className="dashboard-grid">
      <Panel title="Codex runs" meta={`${project.runs.length} total`}>
        {running.length ? running.map((run) => <RunRow key={run.id} run={run} />) : <p className="muted">No active run.</p>}
        <div className="button-row">
          <button className="button secondary" onClick={() => mutate(() => api.runTask(project.path, "reddit_outreach"))}>Reddit scan</button>
          <button className="button secondary" onClick={() => mutate(() => api.runTask(project.path, "competitor_monitor"))}>Competitor check</button>
        </div>
      </Panel>
      <Panel title="Strategy files" meta="local markdown">
        {project.docs.map((doc) => <DocStatus key={doc.key} doc={doc} />)}
      </Panel>
      <Panel title="Activity feed" meta="latest events">
        <EventList events={project.events.slice(0, 7)} />
      </Panel>
      <Panel title="Draft review" meta={`${pendingDrafts.length} pending`}>
        {pendingDrafts.slice(0, 4).map((draft) => <DraftSummary key={draft.id} draft={draft} />)}
        {pendingDrafts.length === 0 ? <p className="muted">No drafts waiting for review.</p> : null}
      </Panel>
      <Panel title="Schedules" meta="project automations">
        {project.schedules.map((schedule) => <ScheduleRow key={schedule.id} schedule={schedule} projectPath={project.path} mutate={mutate} />)}
      </Panel>
    </div>
  );
}

function ContextDocs({ project, mutate }: { project: ProjectStateDto; mutate: (action: () => Promise<unknown>) => Promise<void> }) {
  const [activeKey, setActiveKey] = React.useState(project.docs[0]?.key ?? "brand");
  const active = project.docs.find((doc) => doc.key === activeKey) ?? project.docs[0];
  return (
    <div className="context-layout">
      <div className="doc-tabs">{project.docs.map((doc) => <button key={doc.key} className={doc.key === activeKey ? "active" : ""} onClick={() => setActiveKey(doc.key)}>{doc.fileName}</button>)}</div>
      {active ? <DocEditor key={active.key} doc={active} projectPath={project.path} mutate={mutate} /> : null}
    </div>
  );
}

function DocEditor({ doc, projectPath, mutate }: { doc: DocDto; projectPath: string; mutate: (action: () => Promise<unknown>) => Promise<void> }) {
  const [content, setContent] = React.useState(doc.content);
  return (
    <Panel title={doc.title} meta={doc.fileName}>
      <div className="editor-actions">
        <button className="button secondary" onClick={() => setContent(doc.content)}>Reset</button>
        <button className="button secondary" onClick={() => mutate(() => api.runTask(projectPath, "context_review", undefined, `Improve ${doc.fileName} based on all project context.`))}>Improve with Codex</button>
        <button className="button primary" onClick={() => mutate(() => api.saveDoc(projectPath, doc.key, content))}>Save</button>
      </div>
      <div className="markdown-split">
        <textarea value={content} onChange={(event) => setContent(event.target.value)} spellCheck={false} />
        <article className="markdown-preview"><ReactMarkdown>{content}</ReactMarkdown></article>
      </div>
    </Panel>
  );
}

function Drafts({ project, mutate }: { project: ProjectStateDto; mutate: (action: () => Promise<unknown>) => Promise<void> }) {
  return <div className="stack">{project.drafts.length === 0 ? <Panel title="Drafts" meta="review queue"><p className="muted">No drafts yet. Run Reddit outreach to create reviewable drafts.</p></Panel> : project.drafts.map((draft) => <DraftCard key={draft.id} draft={draft} projectPath={project.path} mutate={mutate} />)}</div>;
}

function DraftCard({ draft, projectPath, mutate }: { draft: DraftDto; projectPath: string; mutate: (action: () => Promise<unknown>) => Promise<void> }) {
  const [body, setBody] = React.useState(draft.body);
  return (
    <Panel title={draft.title} meta={`${draft.channel} · ${formatDate(draft.createdAt)}`}>
      <div className="draft-meta"><span className={statusClass(draft.status)}>{draft.status}</span>{draft.sourceUrl ? <a href={draft.sourceUrl}>{draft.sourceUrl}</a> : null}</div>
      <textarea className="draft-editor" value={body} onChange={(event) => setBody(event.target.value)} />
      <div className="button-row">
        <button className="button secondary" onClick={() => mutate(() => api.saveDraft(projectPath, draft.id, body))}>Save edit</button>
        <button className="button secondary danger" onClick={() => mutate(() => api.rejectDraft(projectPath, draft.id))}>Reject</button>
        <button className="button primary" onClick={() => mutate(() => api.approveDraft(projectPath, draft.id))}>Approve, copy, open source</button>
      </div>
    </Panel>
  );
}

function Automations({ project, mutate }: { project: ProjectStateDto; mutate: (action: () => Promise<unknown>) => Promise<void> }) {
  return <div className="stack">{project.schedules.map((schedule) => <ScheduleEditor key={schedule.id} schedule={schedule} projectPath={project.path} mutate={mutate} />)}</div>;
}

function ScheduleEditor({ schedule, projectPath, mutate }: { schedule: ScheduleManifest; projectPath: string; mutate: (action: () => Promise<unknown>) => Promise<void> }) {
  const [draft, setDraft] = React.useState(schedule);
  return (
    <Panel title={schedule.name} meta={schedule.id}>
      <div className="schedule-form">
        <label>Cadence<select value={draft.cadence} onChange={(event) => setDraft({ ...draft, cadence: event.target.value as ScheduleManifest["cadence"] })}><option value="daily">Daily</option><option value="weekly">Weekly</option><option value="three_times_weekly">Three times weekly</option></select></label>
        <label>Time<input value={draft.timeOfDay} onChange={(event) => setDraft({ ...draft, timeOfDay: event.target.value })} /></label>
        <label>Weekday<select value={draft.dayOfWeek ?? 1} onChange={(event) => setDraft({ ...draft, dayOfWeek: Number(event.target.value) })}><option value={1}>Monday</option><option value={2}>Tuesday</option><option value={3}>Wednesday</option><option value={4}>Thursday</option><option value={5}>Friday</option></select></label>
      </div>
      <p className="muted">Next run: {formatDate(schedule.nextRunAt)} · Last run: {formatDate(schedule.lastRunAt)}</p>
      <div className="button-row">
        <button className="button secondary" onClick={() => mutate(() => api.setScheduleEnabled(projectPath, schedule.id, !schedule.enabled))}>{schedule.enabled ? "Pause" : "Enable"}</button>
        <button className="button secondary" onClick={() => mutate(() => api.updateSchedule(projectPath, draft))}>Save schedule</button>
        <button className="button primary" onClick={() => mutate(() => api.runTask(projectPath, schedule.taskKind, schedule.id))}>Run now</button>
      </div>
    </Panel>
  );
}

function Events({ project }: { project: ProjectStateDto }) {
  return <Panel title="Events" meta={`${project.events.length} loaded`}><EventList events={project.events} /></Panel>;
}

function Settings({ project }: { project: ProjectStateDto }) {
  return (
    <Panel title="Project settings" meta="local workspace">
      <dl className="settings-list"><dt>Name</dt><dd>{project.config.name}</dd><dt>Website</dt><dd>{project.config.websiteUrl}</dd><dt>Repository</dt><dd>{project.config.repoUrl || "Not provided"}</dd><dt>Workspace</dt><dd><code>{project.path}</code></dd><dt>SQLite</dt><dd><code>{project.path}/.gtm-agent/gtm.sqlite</code></dd></dl>
    </Panel>
  );
}

function Panel({ title, meta, children }: { title: string; meta?: string; children: React.ReactNode }) {
  return <section className="panel"><header><h2>{title}</h2>{meta ? <code>{meta}</code> : null}</header>{children}</section>;
}

function DocStatus({ doc }: { doc: DocDto }) { return <div className="row"><strong>{doc.fileName}</strong><span>{doc.content.includes("pending Codex analysis") ? "Needs analysis" : "Ready"}</span><code>{formatDate(doc.updatedAt)}</code></div>; }
function DraftSummary({ draft }: { draft: DraftDto }) { return <div className="row"><strong>{draft.title}</strong><span>{draft.channel}</span><span className={statusClass(draft.status)}>{draft.status}</span></div>; }
function RunRow({ run }: { run: ProjectStateDto["runs"][number] }) { return <div className="row"><strong>{label(run.taskKind)}</strong><span className={statusClass(run.status)}>{run.status}</span><code>{run.id}</code></div>; }
function ScheduleRow({ schedule, projectPath, mutate }: { schedule: ScheduleManifest; projectPath: string; mutate: (action: () => Promise<unknown>) => Promise<void> }) { return <div className="row"><strong>{schedule.name}</strong><span className={schedule.enabled ? "status success" : "status"}>{schedule.enabled ? "enabled" : "paused"}</span><button className="link-button" onClick={() => mutate(() => api.runTask(projectPath, schedule.taskKind, schedule.id))}>Run now</button></div>; }

function EventList({ events }: { events: ProjectStateDto["events"] }) {
  if (events.length === 0) return <p className="muted">No events yet.</p>;
  return <div className="event-list">{events.map((event) => <div className="event-row" key={event.id}><code>{formatDate(event.createdAt)}</code><strong>{event.eventType}</strong><span>{event.summary}</span></div>)}</div>;
}

function viewTitle(view: ViewKey) { return nav.find((item) => item.key === view)?.label ?? "Overview"; }

createRoot(document.getElementById("root")!).render(<App />);
