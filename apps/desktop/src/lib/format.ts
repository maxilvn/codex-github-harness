export function formatDate(value?: string | null) {
  if (!value) return "—";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function label(value: string) {
  return value.replace(/_/g, " ").replace(/\b\w/g, (char: string) => char.toUpperCase());
}

export function statusClass(status: string) {
  if (["completed", "approved", "enabled"].includes(status)) return "status success";
  if (["failed", "rejected"].includes(status)) return "status danger";
  if (["running", "pending", "draft"].includes(status)) return "status warning";
  return "status";
}
