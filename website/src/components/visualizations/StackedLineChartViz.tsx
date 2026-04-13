import React, { useMemo, useRef, useState, useEffect } from 'react';
import { LineChart, Line, XAxis, YAxis, Tooltip, Legend } from 'recharts';
import type { VizProps } from './types';
import { SERIES_COLORS, formatValue } from './utils';

export function StackedLineChartViz({ rows, visualization }: VizProps) {
  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  const xKey = visualization?.x_axis ?? Object.keys(rows[0])[0];
  const yKey = visualization?.y_axis ?? Object.keys(rows[0])[1];
  const seriesKey = visualization?.series_key ?? Object.keys(rows[0])[2] ?? 'series';

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
  const [width, setWidth] = useState(560);
  useEffect(() => {
    if (containerRef.current) setWidth(containerRef.current.offsetWidth || 560);
  }, []);

  return (
    <div ref={containerRef} style={{ overflowX: 'auto' }}>
      <LineChart width={width} height={280} data={data} margin={{ top: 8, right: 12, left: 0, bottom: 8 }}>
        <XAxis
          dataKey="x"
          tick={{ fill: '#9CA3AF', fontSize: 10 }}
          axisLine={{ stroke: '#374151' }}
          tickLine={false}
        />
        <YAxis
          tick={{ fill: '#9CA3AF', fontSize: 10 }}
          axisLine={false}
          tickLine={false}
          width={48}
        />
        <Tooltip
          contentStyle={{ background: '#1F2937', border: '1px solid #374151', borderRadius: 6, fontSize: 12 }}
          labelStyle={{ color: '#D1D5DB' }}
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
  );
}
