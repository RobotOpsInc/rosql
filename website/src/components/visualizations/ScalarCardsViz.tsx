import React from 'react';
import type { VizProps } from './types';
import { formatValue } from './utils';

export function ScalarCardsViz({ rows }: VizProps) {
  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  const row = rows[0];
  const entries = Object.entries(row);

  return (
    <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', padding: '8px 0' }}>
      {entries.map(([key, val]) => (
        <div
          key={key}
          style={{
            background: '#161616',
            border: '1px solid #374151',
            borderRadius: 8,
            padding: '16px 20px',
            minWidth: 120,
            flex: '1 1 120px',
            maxWidth: 200,
          }}
        >
          <div
            style={{
              fontSize: 28,
              fontWeight: 700,
              fontFamily: 'var(--ifm-font-family-monospace)',
              color: '#6EE7B7',
              lineHeight: 1.2,
            }}
          >
            {formatValue(val)}
          </div>
          <div style={{ fontSize: 11, color: '#6B7280', marginTop: 4, textTransform: 'uppercase', letterSpacing: '0.05em' }}>
            {key.replace(/_/g, ' ')}
          </div>
        </div>
      ))}
    </div>
  );
}
