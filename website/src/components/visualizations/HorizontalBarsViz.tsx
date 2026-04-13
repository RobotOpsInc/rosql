import React from 'react';
import type { VizProps } from './types';
import { formatDuration, formatValue } from './utils';

export function HorizontalBarsViz({ rows, visualization }: VizProps) {
  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  const labelKey = visualization?.x_axis ?? 'span_name';
  const valueKey = visualization?.y_axis ?? 'avg_duration';
  // Also show max if present
  const maxKey = 'max_duration';

  const sorted = [...rows].sort((a, b) => Number(b[valueKey]) - Number(a[valueKey]));
  const maxVal = Math.max(...sorted.map((r) => Number(r[valueKey])));

  function barColor(pct: number): string {
    if (pct > 0.7) return '#EF4444';
    if (pct > 0.4) return '#F59E0B';
    return '#6EE7B7';
  }

  // Detect nanosecond values (threshold: values > 1,000,000 are likely ns)
  const isNs = maxVal > 1_000_000;

  function fmt(v: number) {
    return isNs ? formatDuration(v) : formatValue(v);
  }

  return (
    <div style={{ overflowY: 'auto', maxHeight: 300, padding: '4px 0' }}>
      {sorted.map((row, i) => {
        const label = String(row[labelKey] ?? '');
        const val = Number(row[valueKey]);
        const pct = maxVal > 0 ? val / maxVal : 0;
        const color = barColor(pct);
        const maxVal2 = row[maxKey] !== undefined ? Number(row[maxKey]) : null;

        return (
          <div key={i} style={{ marginBottom: 8 }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 3 }}>
              <span style={{ fontSize: 11, color: '#9CA3AF', fontFamily: 'var(--ifm-font-family-monospace)', flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', paddingRight: 8 }}>
                {label}
              </span>
              <span style={{ fontSize: 11, color, fontFamily: 'var(--ifm-font-family-monospace)', flexShrink: 0 }}>
                {fmt(val)}{maxVal2 !== null ? ` / max ${fmt(maxVal2)}` : ''}
              </span>
            </div>
            <div style={{ height: 6, background: '#1F2937', borderRadius: 3, overflow: 'hidden' }}>
              <div
                style={{
                  height: '100%',
                  width: `${(pct * 100).toFixed(1)}%`,
                  background: color,
                  borderRadius: 3,
                  transition: 'width 0.4s ease',
                }}
              />
            </div>
          </div>
        );
      })}
    </div>
  );
}
