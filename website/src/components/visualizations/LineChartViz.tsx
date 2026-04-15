import React, { useRef, useState, useEffect } from 'react';
import { XAxis, YAxis, Tooltip, ReferenceLine, Area, AreaChart } from 'recharts';
import type { VizProps } from './types';
import { formatValue, humanizeYKey, formatEpochTick } from './utils';

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

export function LineChartViz({ rows, visualization }: VizProps) {
  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  const xKey = visualization?.x_axis ?? Object.keys(rows[0])[0];
  const yKey = visualization?.y_axis ?? Object.keys(rows[0])[1];
  const isDeviation = yKey === 'lateral_deviation_m';
  const yLabel = humanizeYKey(yKey);
  const formatXTick = (v: unknown) => formatEpochTick(v, true);

  const data = rows.map((row) => ({
    x: row[xKey],
    y: Number(row[yKey]),
    ...row,
  }));

  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(520);
  useEffect(() => {
    if (containerRef.current) setWidth(containerRef.current.offsetWidth || 520);
  }, []);

  return (
    <div style={{ display: 'flex', alignItems: 'center' }}>
      <div style={Y_LABEL_STYLE}>{yLabel}</div>
      <div ref={containerRef} style={{ flex: 1, overflowX: 'auto' }}>
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
            width={44}
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
    </div>
  );
}
