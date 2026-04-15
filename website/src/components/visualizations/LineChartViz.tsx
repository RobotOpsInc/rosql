import React, { useRef, useState, useEffect } from 'react';
import { XAxis, YAxis, Tooltip, ReferenceLine, Area, AreaChart } from 'recharts';
import type { VizProps } from './types';
import { formatValue } from './utils';

export function LineChartViz({ rows, visualization }: VizProps) {
  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  const xKey = visualization?.x_axis ?? Object.keys(rows[0])[0];
  const yKey = visualization?.y_axis ?? Object.keys(rows[0])[1];
  const isDeviation = yKey === 'lateral_deviation_m';

  // Detect epoch timestamps — DuckDB can return ms, μs, or ns depending on column type.
  // Year-2000 in ms ≈ 9.47e11. Any integer above that is almost certainly an epoch timestamp.
  const EPOCH_2000_MS = 946684800_000; // 9.47e11

  const formatXTick = (v: unknown): string => {
    if (typeof v === 'number' && v > EPOCH_2000_MS) {
      // Infer unit by magnitude: ns > 1e17, μs > 1e14, otherwise ms
      const ms = v > 1e17 ? v / 1_000_000 : v > 1e14 ? v / 1_000 : v;
      return new Date(ms).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
    }
    return String(v);
  };

  const data = rows.map((row) => ({
    x: row[xKey],
    y: Number(row[yKey]),
    ...row,
  }));

  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(560);
  useEffect(() => {
    if (containerRef.current) setWidth(containerRef.current.offsetWidth || 560);
  }, []);

  return (
    <div ref={containerRef} style={{ overflowX: 'auto' }}>
      <AreaChart width={width} height={260} data={data} margin={{ top: 8, right: 12, left: 0, bottom: 8 }}>
        <defs>
          <linearGradient id="lineGrad" x1="0" y1="0" x2="0" y2="1">
            <stop offset="5%" stopColor="#6EE7B7" stopOpacity={0.3} />
            <stop offset="95%" stopColor="#6EE7B7" stopOpacity={0} />
          </linearGradient>
        </defs>
        <XAxis
          dataKey="x"
          tick={{ fill: '#9CA3AF', fontSize: 10 }}
          axisLine={{ stroke: '#374151' }}
          tickLine={false}
          tickFormatter={formatXTick}
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
          itemStyle={{ color: '#6EE7B7' }}
          labelFormatter={formatXTick}
          formatter={(v: number) => [formatValue(v), yKey]}
        />
        {isDeviation && (
          <ReferenceLine y={0} stroke="#374151" strokeDasharray="4 4" label={{ value: 'plan', fill: '#6B7280', fontSize: 10 }} />
        )}
        <Area
          type="monotone"
          dataKey="y"
          name={yKey}
          stroke="#6EE7B7"
          strokeWidth={2}
          fill="url(#lineGrad)"
          dot={false}
          activeDot={{ r: 4, fill: '#6EE7B7' }}
        />
      </AreaChart>
    </div>
  );
}
