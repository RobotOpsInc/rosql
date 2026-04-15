import React from 'react';
import type { VizProps } from './types';

export function RecordingListViz({ rows }: VizProps) {
  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No recordings found</div>;
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8, padding: '4px 0', maxHeight: 300, overflowY: 'auto' }}>
      {rows.map((row, i) => {
        const robotId = String(row['robot_id'] ?? '');
        const sessionId = String(row['session_id'] ?? '');
        const s3Key = String(row['s3_key'] ?? '');
        const startTime = String(row['start_time'] ?? '').slice(0, 19);
        const endTime = String(row['end_time'] ?? '').slice(0, 19);
        const topics = row['topics'];
        const topicList: string[] = Array.isArray(topics) ? topics.map(String)
          : typeof topics === 'string' ? [topics] : [];

        return (
          <div
            key={i}
            style={{
              background: '#161616',
              border: '1px solid #374151',
              borderRadius: 6,
              padding: '10px 14px',
            }}
          >
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: 6 }}>
              <span style={{ fontWeight: 700, color: '#6EE7B7', fontSize: 13, fontFamily: 'var(--ifm-font-family-monospace)' }}>
                {robotId}
              </span>
              <span style={{ fontSize: 10, color: '#6B7280', fontFamily: 'var(--ifm-font-family-monospace)' }}>
                {sessionId}
              </span>
            </div>
            <div style={{ fontSize: 11, color: '#9CA3AF', marginBottom: 6 }}>
              {startTime} → {endTime}
            </div>
            {s3Key && (
              <div style={{ fontSize: 10, color: '#6B7280', fontFamily: 'var(--ifm-font-family-monospace)', marginBottom: 6, wordBreak: 'break-all' }}>
                {s3Key}
              </div>
            )}
            {topicList.length > 0 && (
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
                {topicList.map((t, j) => (
                  <span
                    key={j}
                    style={{
                      fontSize: 10,
                      background: '#1F2937',
                      color: '#9CA3AF',
                      borderRadius: 3,
                      padding: '1px 5px',
                      fontFamily: 'var(--ifm-font-family-monospace)',
                    }}
                  >
                    {t}
                  </span>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
