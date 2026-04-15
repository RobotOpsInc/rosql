import React, { lazy, Suspense } from 'react';
import type { VisualizationConfig } from './types';

const DataTable         = lazy(() => import('./DataTable').then((m) => ({ default: m.DataTable })));
const ScalarCardsViz    = lazy(() => import('./ScalarCardsViz').then((m) => ({ default: m.ScalarCardsViz })));
const LogTableViz       = lazy(() => import('./LogTableViz').then((m) => ({ default: m.LogTableViz })));
const RecordingListViz  = lazy(() => import('./RecordingListViz').then((m) => ({ default: m.RecordingListViz })));
const BarChartViz       = lazy(() => import('./BarChartViz').then((m) => ({ default: m.BarChartViz })));
const HorizontalBarsViz = lazy(() => import('./HorizontalBarsViz').then((m) => ({ default: m.HorizontalBarsViz })));
const LineChartViz      = lazy(() => import('./LineChartViz').then((m) => ({ default: m.LineChartViz })));
const StackedLineChart  = lazy(() => import('./StackedLineChartViz').then((m) => ({ default: m.StackedLineChartViz })));
const GanttViz          = lazy(() => import('./GanttViz').then((m) => ({ default: m.GanttViz })));
const DirectedGraphViz  = lazy(() => import('./DirectedGraphViz').then((m) => ({ default: m.DirectedGraphViz })));
const NodeGraphViz      = lazy(() => import('./NodeGraphViz').then((m) => ({ default: m.NodeGraphViz })));

interface ResultRendererProps {
  rows: Record<string, unknown>[];
  formatHint: string;
  visualization?: VisualizationConfig;
}

const Fallback = <div style={{ color: '#6B7280', padding: '8px 0', fontSize: 12 }}>Rendering…</div>;

export function ResultRenderer({ rows, formatHint, visualization }: ResultRendererProps) {
  const props = { rows, visualization };

  return (
    <Suspense fallback={Fallback}>
      {(() => {
        switch (formatHint) {
          case 'Gantt':            return <GanttViz {...props} />;
          case 'LineChart':        return <LineChartViz {...props} />;
          case 'StackedLineChart': return <StackedLineChart {...props} />;
          case 'BarChart':         return <BarChartViz {...props} />;
          case 'HorizontalBars':   return <HorizontalBarsViz {...props} />;
          case 'DirectedGraph':    return <DirectedGraphViz {...props} />;
          case 'NodeGraph':        return <NodeGraphViz {...props} />;
          case 'ScalarCards':      return <ScalarCardsViz {...props} />;
          case 'LogTable':         return <LogTableViz {...props} />;
          case 'RecordingList':    return <RecordingListViz {...props} />;
          case 'Table':
          default:                 return <DataTable {...props} />;
        }
      })()}
    </Suspense>
  );
}
