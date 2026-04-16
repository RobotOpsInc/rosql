import React, { useRef, useState, useEffect } from 'react';
import { BarChart, Bar, XAxis, YAxis, Tooltip, Cell } from 'recharts';
import type { VizProps } from './types';
import { formatValue, SERIES_COLORS } from './utils';

export function BarChartViz({ rows, visualization }: VizProps) {
  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  const xKey = visualization?.x_axis ?? Object.keys(rows[0])[0];
  const yKey = visualization?.y_axis ?? Object.keys(rows[0])[1];

  const data = rows.map((row) => ({ x: row[xKey], y: Number(row[yKey]) }));
  const containerRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(560);
  useEffect(() => {
    if (containerRef.current) setWidth(containerRef.current.offsetWidth || 560);
  }, []);

  return (
    <div ref={containerRef} style={{ overflowX: 'auto' }}>
      <BarChart width={width} height={280} data={data} margin={{ top: 8, right: 12, left: 0, bottom: 32 }}>
        <XAxis
          dataKey="x"
          tick={{ fill: '#9CA3AF', fontSize: 11 }}
          axisLine={{ stroke: '#374151' }}
          tickLine={false}
          angle={-30}
          textAnchor="end"
        />
        <YAxis
          tick={{ fill: '#9CA3AF', fontSize: 11 }}
          axisLine={false}
          tickLine={false}
          width={48}
        />
        <Tooltip
          contentStyle={{ background: '#1F2937', border: '1px solid #374151', borderRadius: 6, fontSize: 12 }}
          labelStyle={{ color: '#D1D5DB' }}
          itemStyle={{ color: '#6EE7B7' }}
          formatter={(v: number) => formatValue(v)}
        />
        <Bar dataKey="y" name={yKey} radius={[3, 3, 0, 0]}>
          {data.map((_, i) => (
            <Cell key={i} fill={SERIES_COLORS[i % SERIES_COLORS.length]} />
          ))}
        </Bar>
      </BarChart>
    </div>
  );
}
