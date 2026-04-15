import React, { useState } from 'react';
import type { VizProps } from './types';
import { formatEpochTick } from './utils';

const SEVERITY_COLORS: Record<string, string> = {
  ERROR: '#EF4444',
  WARN:  '#F59E0B',
  WARNING: '#F59E0B',
  INFO:  '#3B82F6',
  DEBUG: '#6B7280',
};

function severityColor(sev: string): string {
  return SEVERITY_COLORS[String(sev).toUpperCase()] ?? '#6B7280';
}

export function LogTableViz({ rows }: VizProps) {
  const [expanded, setExpanded] = useState<number | null>(null);

  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  return (
    <div style={{ overflowY: 'auto', maxHeight: 300 }}>
      {rows.map((row, i) => {
        const sev = String(row['severity_text'] ?? row['severity'] ?? 'INFO');
        const color = severityColor(sev);
        const body = String(row['body'] ?? row['message'] ?? '');
        const rawTs = row['timestamp'];
        const ts = formatEpochTick(typeof rawTs === 'number' ? rawTs : Number(rawTs), true);
        const service = String(row['service_name'] ?? '');
        const isOpen = expanded === i;

        return (
          <div
            key={i}
            onClick={() => setExpanded(isOpen ? null : i)}
            style={{
              display: 'flex',
              gap: 10,
              padding: '7px 10px',
              borderBottom: '1px solid #1F2937',
              cursor: 'pointer',
              background: i % 2 === 0 ? 'transparent' : 'rgba(255,255,255,0.02)',
            }}
          >
            {/* Severity band */}
            <div style={{ width: 3, borderRadius: 2, background: color, flexShrink: 0 }} />
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 2 }}>
                <span style={{ fontSize: 10, fontWeight: 700, color, minWidth: 40 }}>{sev}</span>
                <span style={{ fontSize: 10, color: '#6B7280', flexShrink: 0 }}>{ts}</span>
                {service && <span style={{ fontSize: 10, color: '#9CA3AF' }}>{service}</span>}
              </div>
              <div
                style={{
                  fontSize: 12,
                  color: '#D1D5DB',
                  fontFamily: 'var(--ifm-font-family-monospace)',
                  overflow: isOpen ? 'visible' : 'hidden',
                  textOverflow: 'ellipsis',
                  whiteSpace: isOpen ? 'pre-wrap' : 'nowrap',
                }}
              >
                {body}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
