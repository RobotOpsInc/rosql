// Shared utilities for visualization components

export const SERIES_COLORS = [
  '#6EE7B7', // green (primary accent)
  '#60A5FA', // blue
  '#F472B6', // pink
  '#FBBF24', // amber
  '#A78BFA', // violet
  '#34D399', // emerald
  '#F87171', // red
  '#38BDF8', // sky
];

export function formatDuration(ns: number): string {
  if (ns >= 1_000_000_000) return `${(ns / 1_000_000_000).toFixed(1)}s`;
  if (ns >= 1_000_000) return `${(ns / 1_000_000).toFixed(0)}ms`;
  if (ns >= 1_000) return `${(ns / 1_000).toFixed(0)}µs`;
  return `${ns}ns`;
}

export function formatValue(v: unknown): string {
  if (v === null || v === undefined) return '—';
  if (typeof v === 'number') {
    if (!isFinite(v)) return '—';
    return Number.isInteger(v) ? String(v) : v.toFixed(2);
  }
  return String(v);
}

// Known Y-axis labels — anything not listed gets auto-humanized from snake_case.
const Y_AXIS_LABELS: Record<string, string> = {
  lateral_deviation_m: 'Lateral path deviation (m)',
  avg_duration: 'Avg duration (ns)',
  count: 'Count',
};

/** Return a human-readable Y-axis label for a column key. */
export function humanizeYKey(key: string): string {
  return Y_AXIS_LABELS[key] ?? key.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
}

/**
 * Format an epoch value (ms, μs, or ns) as HH:MM or HH:MM:SS.
 * Returns the raw string for non-timestamp values.
 */
export function formatEpochTick(v: unknown, includeSeconds = true): string {
  // Recharts may pass tick values as strings — coerce to number first.
  const n = typeof v === 'number' ? v : Number(v);
  if (isNaN(n) || n <= 946684800_000) return String(v);
  // Infer unit by magnitude: ns > 1e17, μs > 1e14, otherwise ms
  const ms = n > 1e17 ? n / 1_000_000 : n > 1e14 ? n / 1_000 : n;
  const opts: Intl.DateTimeFormatOptions = includeSeconds
    ? { hour: '2-digit', minute: '2-digit', second: '2-digit' }
    : { hour: '2-digit', minute: '2-digit' };
  return new Date(ms).toLocaleTimeString([], opts);
}

/** Pivot flat rows by a series key into { seriesKey: [{x, y}, ...] } */
export function pivotBySeries(
  rows: Record<string, unknown>[],
  xKey: string,
  yKey: string,
  seriesKey: string,
): Record<string, { x: unknown; y: number }[]> {
  const result: Record<string, { x: unknown; y: number }[]> = {};
  for (const row of rows) {
    const series = String(row[seriesKey] ?? 'unknown');
    if (!result[series]) result[series] = [];
    const y = Number(row[yKey]);
    if (!isNaN(y)) result[series].push({ x: row[xKey], y });
  }
  return result;
}
