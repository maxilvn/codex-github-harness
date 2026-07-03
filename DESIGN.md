# GTM Agent Design System

> A restrained OpenAI/Vercel-inspired product interface for a local Codex-powered GTM control plane. This file is the source of truth for UI implementation.

## 1. Visual Direction

GTM Agent should feel like a precise local developer tool, not a marketing SaaS. The interface is monochrome-first, spacious, sharp, and readable. It uses a white canvas, quiet gray structure, strong black typography, compact controls, and transparent data surfaces.

Core attributes:

- Local, inspectable, file-backed, trustworthy.
- Clean dashboard density with no decorative filler.
- Light mode first.
- Monochrome system with blue used only for focus, links, or active state.
- Strong text hierarchy, thin borders, subtle surface contrast.

Avoid:

- Gradients as decoration.
- Glassmorphism.
- Heavy shadows.
- Colorful status cards.
- Marketing hero sections inside the product UI.
- Placeholder-looking skeletons or fake inert controls.

## 2. Color Tokens

| Token | Hex | Use |
| --- | --- | --- |
| `--color-bg` | `#ffffff` | Main app background |
| `--color-bg-subtle` | `#fafafa` | Sidebar, panels, table headers |
| `--color-surface` | `#ffffff` | Cards, editors, dialogs |
| `--color-surface-raised` | `#f7f7f7` | Secondary panels |
| `--color-text` | `#171717` | Primary text and primary buttons |
| `--color-text-muted` | `#666666` | Secondary text |
| `--color-text-faint` | `#8f8f8f` | Captions and metadata |
| `--color-border` | `#eaeaea` | Default borders |
| `--color-border-strong` | `#d4d4d4` | Active or structural borders |
| `--color-focus` | `#0070f3` | Focus rings and links |
| `--color-success` | `#0a7a35` | Completed state text only |
| `--color-warning` | `#8a5a00` | Pending/review state text only |
| `--color-danger` | `#b42318` | Failed/rejected state text only |

## 3. Typography

Use Geist-like system typography:

```css
font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
font-family-mono: "SFMono-Regular", "Roboto Mono", Consolas, "Liberation Mono", monospace;
```

Scale:

- Display: 32px / 40px, 650 weight.
- Page title: 24px / 32px, 650 weight.
- Section title: 16px / 24px, 600 weight.
- Body: 14px / 22px, 400 weight.
- UI label: 12px / 16px, 550 weight, optional uppercase tracking 0.04em.
- Mono metadata: 12px / 18px, regular mono.

## 4. Layout

- App shell uses fixed 264px sidebar and fluid content region.
- Global page padding: 28px desktop, 18px tablet, 14px mobile.
- Dashboard max width: 1440px.
- Cards use 1px borders, 12px radius, no heavy shadow.
- Dense lists use 44px minimum row height.
- Editors use 2-column read/edit layout when width allows.

Spacing scale:

`4, 6, 8, 10, 12, 16, 20, 24, 28, 32, 40, 56, 72`

## 5. Components

### Buttons

- Primary: black background, white text, 8px radius, 32-36px height.
- Secondary: white background, gray border, black text.
- Ghost: transparent, muted text, subtle hover background.
- Destructive: white background, red text, red-tinted border only when needed.

### Panels

- 1px gray border.
- 12px radius.
- Header with title and optional mono metadata.
- No nested card stacks unless the nested item is interactive.

### Sidebar

- `#fafafa` background with right border.
- Simple app mark, project selector, nav list.
- Active nav uses white background, border, black text.

### Markdown docs

- Read mode: prose-like but compact.
- Edit mode: monospace textarea with line height 1.6.
- Save controls remain visible near the editor header.

### Activity feed

- Event rows show type, timestamp, task, and summary.
- Use status text color only; no bright badges.

### Draft queue

- Draft rows show channel, source URL, status, excerpt, and actions.
- Approve copies draft to clipboard and opens source URL.

## 6. Motion and Interaction

- 120ms hover/focus transitions.
- Respect `prefers-reduced-motion`.
- No decorative animations in MVP.
- Live-running tasks update through real polling and persisted events.

## 7. Responsive Behavior

- Desktop: sidebar + multi-column dashboard.
- Tablet: sidebar narrows and dashboard becomes 2 columns.
- Mobile: sidebar becomes top navigation; panels stack vertically.

## 8. Implementation Rules

- No placeholder-only controls. If a visible control exists, it must perform the MVP behavior.
- No direct LLM/API calls from frontend or backend except launching the local Codex CLI.
- Context markdown files are canonical; app state must not replace them.
- All app-launched Codex sessions run in the project workspace and are non-ephemeral.
- UI components must use shared tokens/classes rather than one-off colors.
