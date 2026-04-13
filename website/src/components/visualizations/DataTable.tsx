import React from 'react';
import type { VizProps } from './types';
import { formatValue } from './utils';

export function DataTable({ rows, visualization }: VizProps) {
  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  const colorField = visualization?.color_field;
  const columns = Object.keys(rows[0]);

  return (
    <div style={{ overflowX: 'auto', maxHeight: 300, overflowY: 'auto' }}>
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 12, fontFamily: 'var(--ifm-font-family-monospace)', color: '#D1D5DB' }}>
        <thead>
          <tr style={{ borderBottom: '1px solid #374151', position: 'sticky', top: 0, background: '#161616' }}>
            {columns.map((col) => (
              <th key={col} style={{ padding: '6px 10px', textAlign: 'left', color: '#9CA3AF', fontWeight: 600, whiteSpace: 'nowrap' }}>
                {col}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => {
            const isAnomaly = colorField ? Boolean(row[colorField]) : false;
            const rowBg = i % 2 === 0 ? 'transparent' : 'rgba(255,255,255,0.02)';
            return (
              <tr key={i} style={{ background: rowBg, borderBottom: '1px solid #1F2937' }}>
                {columns.map((col) => {
                  const isColoredCell = col === colorField;
                  const val = row[col];
                  const cellColor = isColoredCell
                    ? val ? '#FCA5A5' : '#6EE7B7'
                    : undefined;
                  return (
                    <td
                      key={col}
                      style={{
                        padding: '5px 10px',
                        whiteSpace: 'nowrap',
                        color: cellColor,
                        fontWeight: isColoredCell ? 600 : undefined,
                      }}
                    >
                      {isColoredCell ? (val ? 'true' : 'false') : formatValue(val)}
                    </td>
                  );
                })}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
