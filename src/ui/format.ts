/** Small formatting helpers for timestamps (ms epoch). */

export function formatDate(ms: number): string {
  return new Date(ms).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

export function formatDateTime(ms: number): string {
  return new Date(ms).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
  });
}

/** Relative time like "just now", "3h ago", "2d ago", or a date for older items. */
export function relativeTime(ms: number, now: number = Date.now()): string {
  const diff = now - ms;
  const sec = Math.round(diff / 1000);
  if (sec < 45) return "just now";
  const min = Math.round(sec / 60);
  if (min < 60) return `${min}m ago`;
  const hr = Math.round(min / 60);
  if (hr < 24) return `${hr}h ago`;
  const day = Math.round(hr / 24);
  if (day < 7) return `${day}d ago`;
  return formatDate(ms);
}

/** When a card is next due, phrased for a person. */
export function dueLabel(dueAt: number, now: number = Date.now()): string {
  if (dueAt <= now) return "due now";
  const diff = dueAt - now;
  const min = Math.round(diff / 60000);
  if (min < 60) return `in ${min}m`;
  const hr = Math.round(min / 60);
  if (hr < 24) return `in ${hr}h`;
  const day = Math.round(hr / 24);
  if (day < 30) return `in ${day}d`;
  const mo = Math.round(day / 30);
  if (mo < 12) return `in ${mo}mo`;
  return `in ${(day / 365).toFixed(1)}y`;
}
