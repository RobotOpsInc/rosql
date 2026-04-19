import React, { useMemo, useState } from 'react';
import type { VizProps } from './types';
import { formatDuration } from './utils';
import { LogTableViz } from './LogTableViz';

const ROW_HEIGHT = 32;
const LABEL_WIDTH = 220;
const PADDING = 8;

interface SpanRow {
  span_id: string;
  parent_span_id: string;
  span_name: string;
  service_name: string;
  duration: number;
  status_code: string;
  depth: number;
  start: number; // relative to trace start (0-based ns)
}

interface LogRow {
  trace_id?: string;
  span_id?: string;
  severity_text?: string;
  body?: string;
  timestamp?: string;
}

function buildTree(rows: Record<string, unknown>[]): SpanRow[] {
  const spanMap = new Map<string, SpanRow>();
  const childrenOf = new Map<string, string[]>();

  // Build map
  for (const row of rows) {
    const sid = String(row['span_id'] ?? '');
    const pid = String(row['parent_span_id'] ?? '');
    spanMap.set(sid, {
      span_id: sid,
      parent_span_id: pid,
      span_name: String(row['span_name'] ?? row['span_name_col'] ?? ''),
      service_name: String(row['service_name'] ?? ''),
      duration: Number(row['duration'] ?? 0),
      status_code: String(row['status_code'] ?? 'OK'),
      depth: 0,
      start: 0,
    });
    if (!childrenOf.has(pid)) childrenOf.set(pid, []);
    childrenOf.get(pid)!.push(sid);
  }

  // Find roots (spans with no parent in the set)
  const spanIds = new Set(spanMap.keys());
  const roots = [...spanMap.values()].filter(
    (s) => !s.parent_span_id || !spanIds.has(s.parent_span_id)
  );

  const result: SpanRow[] = [];
  function visit(sid: string, depth: number, parentStart: number) {
    const span = spanMap.get(sid);
    if (!span) return;
    span.depth = depth;
    span.start = parentStart;
    result.push(span);
    const children = childrenOf.get(sid) ?? [];
    let childOffset = parentStart;
    for (const childId of children) {
      visit(childId, depth + 1, childOffset);
      const child = spanMap.get(childId);
      if (child) childOffset += child.duration;
    }
  }
  for (const root of roots) {
    visit(root.span_id, 0, 0);
  }
  return result;
}

// Returns true if any later span in the DFS order shares depth `d` before a shallower span closes the scope.
function hasMoreSiblingsAt(spans: SpanRow[], i: number, d: number): boolean {
  for (let j = i + 1; j < spans.length; j++) {
    if (spans[j].depth < d) return false;
    if (spans[j].depth === d) return true;
  }
  return false;
}

export function GanttViz({ rows }: VizProps) {
  const [tooltip, setTooltip] = useState<{ span: SpanRow; x: number; y: number } | null>(null);

  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  // Separate trace spans from enrichment (log) rows
  const traceRows = rows.filter((r) => r['span_id'] !== undefined && r['duration'] !== undefined);
  const logRows = rows.filter((r) => r['span_id'] !== undefined && r['severity_text'] !== undefined) as unknown as LogRow[];

  const spans = useMemo(() => buildTree(traceRows), [traceRows]);

  if (spans.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No spans to display</div>;
  }

  const totalNs = Math.max(...spans.map((s) => s.start + s.duration));
  const svgHeight = spans.length * ROW_HEIGHT + PADDING * 2;
  const chartWidth = 560; // inner chart area width

  function xScale(ns: number) {
    return totalNs > 0 ? (ns / totalNs) * chartWidth : 0;
  }

  return (
    <div style={{ overflowX: 'auto', overflowY: 'auto', maxHeight: 320, position: 'relative' }}>
      {/* Legend */}
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 12,
          marginBottom: 6,
          flexWrap: 'wrap',
          fontFamily: 'var(--ifm-font-family-monospace)',
          fontSize: 10,
          color: '#9CA3AF',
        }}
      >
        {([
          { color: '#EF4444', label: 'ERROR span' },
          { color: '#6EE7B7', label: 'OK span' },
        ] as const).map(({ color, label }) => (
          <span key={label} style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
            <span
              style={{
                display: 'inline-block',
                width: 10,
                height: 10,
                borderRadius: 2,
                background: color,
                flexShrink: 0,
              }}
            />
            {label}
          </span>
        ))}
      </div>

      <svg
        width={LABEL_WIDTH + chartWidth + 80}
        height={svgHeight}
        style={{ display: 'block', userSelect: 'none' }}
        onMouseLeave={() => setTooltip(null)}
      >
        {/* Rows */}
        {spans.map((span, i) => {
          const y = PADDING + i * ROW_HEIGHT;
          const barX = LABEL_WIDTH + xScale(span.start);
          const barW = Math.max(xScale(span.duration), 2);
          const isError = span.status_code === 'ERROR';
          const barColor = isError ? '#EF4444' : '#6EE7B7';
          const barAlpha = isError ? 1 : 0.85;

          // Build tree connector lines for this row
          const connectors: React.ReactNode[] = [];
          for (let d = 1; d <= span.depth; d++) {
            const trunkX = PADDING + (d - 1) * 14 + 7;
            const midY = y + ROW_HEIGHT / 2;
            const isElbow = d === span.depth;
            const hasSiblings = hasMoreSiblingsAt(spans, i, d);

            if (isElbow) {
              // Vertical from top of row to mid (the stem of the L)
              connectors.push(
                <line key={`v-${d}`} x1={trunkX} y1={y} x2={trunkX} y2={midY} stroke="#4B5563" strokeWidth={1} />
              );
              // Continue vertical past mid if this span has siblings below
              if (hasSiblings) {
                connectors.push(
                  <line key={`vc-${d}`} x1={trunkX} y1={midY} x2={trunkX} y2={y + ROW_HEIGHT} stroke="#4B5563" strokeWidth={1} />
                );
              }
              // Horizontal arm of the L
              connectors.push(
                <line key={`h-${d}`} x1={trunkX} y1={midY} x2={trunkX + 7} y2={midY} stroke="#4B5563" strokeWidth={1} />
              );
            } else {
              // Continuation trunk for an ancestor that still has descendants below
              if (hasSiblings) {
                connectors.push(
                  <line key={`v-${d}`} x1={trunkX} y1={y} x2={trunkX} y2={y + ROW_HEIGHT} stroke="#4B5563" strokeWidth={1} />
                );
              }
            }
          }

          return (
            <g key={span.span_id}>
              {connectors}
              {/* Label */}
              <text
                x={PADDING + span.depth * 14 + 12}
                y={y + ROW_HEIGHT / 2 + 1}
                textAnchor="start"
                fill="#9CA3AF"
                fontSize={10}
                fontFamily="var(--ifm-font-family-monospace)"
                style={{ pointerEvents: 'none' }}
              >
                {span.span_name.split('/').pop() ?? span.span_name}
              </text>
              {/* Bar */}
              <rect
                x={barX}
                y={y + 6}
                width={barW}
                height={ROW_HEIGHT - 12}
                rx={3}
                fill={barColor}
                fillOpacity={barAlpha}
                style={{ cursor: 'pointer' }}
                onMouseEnter={(e) => setTooltip({ span, x: e.clientX, y: e.clientY })}
              />
              {/* Duration label */}
              {barW > 30 && (
                <text
                  x={barX + barW + 4}
                  y={y + ROW_HEIGHT / 2 + 1}
                  fill={isError ? '#FCA5A5' : '#6EE7B7'}
                  fontSize={9}
                  fontFamily="var(--ifm-font-family-monospace)"
                  style={{ pointerEvents: 'none' }}
                >
                  {formatDuration(span.duration)}
                </text>
              )}
              {/* Divider line */}
              <line
                x1={0}
                y1={y + ROW_HEIGHT}
                x2={LABEL_WIDTH + chartWidth + 80}
                y2={y + ROW_HEIGHT}
                stroke="#1F2937"
                strokeWidth={0.5}
              />
            </g>
          );
        })}

        {/* Vertical separators every ~25% */}
        {[0.25, 0.5, 0.75, 1].map((pct) => (
          <g key={pct}>
            <line
              x1={LABEL_WIDTH + pct * chartWidth}
              y1={PADDING}
              x2={LABEL_WIDTH + pct * chartWidth}
              y2={svgHeight - PADDING}
              stroke="#1F2937"
              strokeWidth={0.5}
              strokeDasharray="3 3"
            />
            <text
              x={LABEL_WIDTH + pct * chartWidth}
              y={PADDING - 2}
              textAnchor="middle"
              fill="#4B5563"
              fontSize={8}
              fontFamily="var(--ifm-font-family-monospace)"
            >
              {formatDuration(pct * totalNs)}
            </text>
          </g>
        ))}
      </svg>

      {/* Log table */}
      {logRows.length > 0 && (
        <div style={{ marginTop: 12, borderTop: '1px solid #1F2937', paddingTop: 8 }}>
          <div
            style={{
              fontSize: 10,
              color: '#4B5563',
              fontFamily: 'var(--ifm-font-family-monospace)',
              marginBottom: 6,
              textTransform: 'uppercase',
              letterSpacing: '0.05em',
            }}
          >
            Logs
          </div>
          <LogTableViz rows={logRows as unknown as Record<string, unknown>[]} />
        </div>
      )}

      {/* Tooltip */}
      {tooltip && (
        <div
          style={{
            position: 'fixed',
            left: tooltip.x + 12,
            top: tooltip.y - 20,
            background: '#1F2937',
            border: '1px solid #374151',
            borderRadius: 6,
            padding: '8px 10px',
            fontSize: 11,
            color: '#D1D5DB',
            zIndex: 999,
            pointerEvents: 'none',
            fontFamily: 'var(--ifm-font-family-monospace)',
            maxWidth: 260,
          }}
        >
          <div style={{ fontWeight: 700, color: '#F9FAFB', marginBottom: 4 }}>{tooltip.span.span_name}</div>
          <div>Duration: <span style={{ color: '#6EE7B7' }}>{formatDuration(tooltip.span.duration)}</span></div>
          <div>Status: <span style={{ color: tooltip.span.status_code === 'ERROR' ? '#EF4444' : '#6EE7B7' }}>{tooltip.span.status_code}</span></div>
          <div style={{ color: '#6B7280', marginTop: 2 }}>{tooltip.span.service_name}</div>
        </div>
      )}
    </div>
  );
}
