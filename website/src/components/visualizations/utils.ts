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
