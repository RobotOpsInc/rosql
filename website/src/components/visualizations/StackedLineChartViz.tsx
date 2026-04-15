import React, { useMemo, useRef, useState, useEffect } from 'react';
import { LineChart, Line, XAxis, YAxis, Tooltip, Legend } from 'recharts';
import type { VizProps } from './types';
import { SERIES_COLORS, formatValue, humanizeYKey, formatEpochTick } from './utils';

const Y_LABEL_STYLE: React.CSSProperties = {
  writingMode: 'vertical-rl',
  transform: 'rotate(180deg)',
  color: '#6B7280',
  fontSize: 9,
  whiteSpace: 'nowrap',
  fontFamily: 'var(--ifm-font-family-monospace)',
  flexShrink: 0,
  paddingRight: 4,
};

export function StackedLineChartViz({ rows, visualization }: VizProps) {
  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  const xKey = visualization?.x_axis ?? Object.keys(rows[0])[0];
  const seriesKey = visualization?.series_key ?? Object.keys(rows[0])[2] ?? 'series';
  // y_axis may be null for pipeline queries — fall back to the first column that is
  // neither the x-axis nor the series key, so we don't accidentally pick the series
  // column as the numeric value to plot.
  const yKey = visualization?.y_axis ??
    Object.keys(rows[0]).find((k) => k !== xKey && k !== seriesKey) ??
    Object.keys(rows[0])[1];
  const yLabel = humanizeYKey(yKey);

  // Collect all series names (in order of first appearance)
  const seriesNames = useMemo(() => {
    const seen = new Set<string>();
    const order: string[] = [];
    for (const row of rows) {
      const s = String(row[seriesKey] ?? 'unknown');
      if (!seen.has(s)) { seen.add(s); order.push(s); }
    }
    return order;
  }, [rows, seriesKey]);

  // Build wide-format data: [{x: ..., 'robot-amr-01': 30.2, 'robot-amr-02': 92.3, ...}, ...]
  const data = useMemo(() => {
    const byX: Record<string, Record<string, number>> = {};
    const xOrder: string[] = [];
    for (const row of rows) {
      const x = String(row[xKey] ?? '');
      const s = String(row[seriesKey] ?? 'unknown');
      const y = Number(row[yKey]);
      if (!byX[x]) { byX[x] = {}; xOrder.push(x); }
      byX[x][s] = y;
    }
    return xOrder.map((x) => ({ x, ...byX[x] }));
  }, [rows, xKey, yKey, seriesKey]);

  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(520);
  useEffect(() => {
    if (containerRef.current) setWidth(containerRef.current.offsetWidth || 520);
  }, []);

  return (
    <div style={{ display: 'flex', alignItems: 'center' }}>
      <div style={Y_LABEL_STYLE}>{yLabel}</div>
      <div ref={containerRef} style={{ flex: 1, overflowX: 'auto' }}>
        <LineChart width={width} height={280} data={data} margin={{ top: 8, right: 12, left: 0, bottom: 8 }}>
          <XAxis
            dataKey="x"
            tick={{ fill: '#9CA3AF', fontSize: 10 }}
            axisLine={{ stroke: '#374151' }}
            tickLine={false}
            tickFormatter={(v) => formatEpochTick(v, false)}
          />
          <YAxis
            tick={{ fill: '#9CA3AF', fontSize: 10 }}
            axisLine={false}
            tickLine={false}
            width={44}
          />
          <Tooltip
            contentStyle={{ background: '#1F2937', border: '1px solid #374151', borderRadius: 6, fontSize: 12 }}
            labelStyle={{ color: '#D1D5DB' }}
            labelFormatter={(v) => formatEpochTick(v, false)}
            formatter={(v: number, name: string) => [formatValue(v), name]}
          />
          <Legend wrapperStyle={{ fontSize: 11, color: '#9CA3AF' }} />
          {seriesNames.map((name, i) => (
            <Line
              key={name}
              type="monotone"
              dataKey={name}
              stroke={SERIES_COLORS[i % SERIES_COLORS.length]}
              strokeWidth={2}
              dot={false}
              activeDot={{ r: 3 }}
            />
          ))}
        </LineChart>
      </div>
    </div>
  );
}
