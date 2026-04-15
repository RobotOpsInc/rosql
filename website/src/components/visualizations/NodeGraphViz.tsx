import React, { useEffect, useState } from 'react';
import type { VizProps } from './types';
import { SvgTooltip } from './SvgTooltip';
import type { SvgTooltipProps } from './SvgTooltip';

interface GraphNode {
  id: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
}

interface GraphEdge {
  source: string;
  target: string;
  label: string;
}

type Tooltip = Omit<SvgTooltipProps, 'svgWidth'>;

const WIDTH = 580;
const HEIGHT = 280;
const NODE_R = 18;

function simpleForce(nodes: GraphNode[], edges: GraphEdge[], iters = 120): GraphNode[] {
  const ns = nodes.map((n) => ({ ...n }));

  for (let i = 0; i < iters; i++) {
    // Repulsion
    for (let a = 0; a < ns.length; a++) {
      for (let b = a + 1; b < ns.length; b++) {
        const dx = ns[b].x - ns[a].x;
        const dy = ns[b].y - ns[a].y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        const f = 7000 / (dist * dist);
        ns[a].vx -= (dx / dist) * f;
        ns[a].vy -= (dy / dist) * f;
        ns[b].vx += (dx / dist) * f;
        ns[b].vy += (dy / dist) * f;
      }
    }
    // Attraction along edges
    for (const e of edges) {
      const s = ns.find((n) => n.id === e.source);
      const t = ns.find((n) => n.id === e.target);
      if (!s || !t) continue;
      const dx = t.x - s.x;
      const dy = t.y - s.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      const f = dist / 60;
      s.vx += (dx / dist) * f;
      s.vy += (dy / dist) * f;
      t.vx -= (dx / dist) * f;
      t.vy -= (dy / dist) * f;
    }
    // Gravity toward center
    for (const n of ns) {
      n.vx += (WIDTH / 2 - n.x) * 0.004;
      n.vy += (HEIGHT / 2 - n.y) * 0.004;
    }
    // Dampen and clamp
    for (const n of ns) {
      n.vx *= 0.85;
      n.vy *= 0.85;
      n.x = Math.max(NODE_R + 4, Math.min(WIDTH - NODE_R - 4, n.x + n.vx));
      n.y = Math.max(NODE_R + 4, Math.min(HEIGHT - NODE_R - 4, n.y + n.vy));
    }
  }
  return ns;
}

export function NodeGraphViz({ rows }: VizProps) {
  const [nodes, setNodes] = useState<GraphNode[]>([]);
  const [edges, setEdges] = useState<GraphEdge[]>([]);
  const [tooltip, setTooltip] = useState<Tooltip | null>(null);

  useEffect(() => {
    if (rows.length === 0) return;

    const nodeIds = new Set<string>();
    const edgeList: GraphEdge[] = [];

    for (const row of rows) {
      const src = String(row['source_node'] ?? '');
      const tgt = String(row['target_node'] ?? '');
      const label = String(row['topic'] ?? '');
      if (src) nodeIds.add(src);
      if (tgt) nodeIds.add(tgt);
      if (src && tgt) edgeList.push({ source: src, target: tgt, label });
    }

    const nodeArr = [...nodeIds].map((id, i, arr) => {
      const angle = (i / arr.length) * 2 * Math.PI;
      const r = Math.min(WIDTH, HEIGHT) / 2 - NODE_R - 20;
      return { id, x: WIDTH / 2 + r * Math.cos(angle), y: HEIGHT / 2 + r * Math.sin(angle), vx: 0, vy: 0 };
    });

    const laid = simpleForce(nodeArr, edgeList, 120);
    setNodes(laid);
    setEdges(edgeList);
  }, [rows]);

  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  const nodeMap = new Map<string, GraphNode>(nodes.map((n) => [n.id, n]));

  return (
    <div style={{ overflowX: 'auto' }}>
      <svg width={WIDTH} height={HEIGHT} style={{ display: 'block' }} onMouseLeave={() => setTooltip(null)}>
        <defs>
          <marker id="ng-arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
            <path d="M0,0 L0,6 L8,3 z" fill="#6EE7B7" />
          </marker>
        </defs>

        {/* Edges */}
        {edges.map((edge, i) => {
          const s = nodeMap.get(edge.source);
          const t = nodeMap.get(edge.target);
          if (!s || !t) return null;
          const dx = t.x - s.x;
          const dy = t.y - s.y;
          const dist = Math.sqrt(dx * dx + dy * dy) || 1;
          const x1 = s.x + (dx / dist) * NODE_R;
          const y1 = s.y + (dy / dist) * NODE_R;
          const x2 = t.x - (dx / dist) * (NODE_R + 6);
          const y2 = t.y - (dy / dist) * (NODE_R + 6);
          const mx = (x1 + x2) / 2;
          const my = (y1 + y2) / 2;
          return (
            <g key={i}>
              <line
                x1={x1} y1={y1} x2={x2} y2={y2}
                stroke="#6EE7B7"
                strokeWidth={1.5}
                strokeOpacity={0.6}
                markerEnd="url(#ng-arrow)"
              />
              {edge.label && (
                <text
                  x={mx} y={my - 4}
                  textAnchor="middle"
                  fill="#6B7280"
                  fontSize={9}
                  fontFamily="var(--ifm-font-family-monospace)"
                  style={{ cursor: 'default' }}
                  onMouseEnter={() => setTooltip({ text: edge.label, cx: mx, cy: my - 12 })}
                  onMouseLeave={() => setTooltip(null)}
                >
                  {edge.label.length > 16 ? edge.label.slice(0, 16) + '…' : edge.label}
                </text>
              )}
            </g>
          );
        })}

        {/* Nodes */}
        {nodes.map((n) => {
          const shortName = n.id.split('/').pop() ?? n.id;
          return (
            <g
              key={n.id}
              style={{ cursor: 'default' }}
              onMouseEnter={() => setTooltip({ text: n.id, cx: n.x, cy: n.y - NODE_R - 6 })}
              onMouseLeave={() => setTooltip(null)}
            >
              <circle cx={n.x} cy={n.y} r={NODE_R} fill="#1F2937" stroke="#6EE7B7" strokeWidth={1.5} />
              <text
                x={n.x}
                y={n.y + 1}
                textAnchor="middle"
                dominantBaseline="middle"
                fill="#D1D5DB"
                fontSize={8}
                fontFamily="var(--ifm-font-family-monospace)"
              >
                {shortName.length > 10 ? shortName.slice(0, 10) + '…' : shortName}
              </text>
            </g>
          );
        })}

        {/* Tooltip — rendered last so it appears on top of everything */}
        {tooltip && <SvgTooltip {...tooltip} svgWidth={WIDTH} />}
      </svg>
    </div>
  );
}
