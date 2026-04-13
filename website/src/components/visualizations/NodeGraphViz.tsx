import React, { useEffect, useState } from 'react';
import type { VizProps } from './types';

interface GraphNode {
  id: string;
  type: 'node' | 'topic';
  x: number;
  y: number;
  vx: number;
  vy: number;
}

interface GraphEdge {
  source: string;
  target: string;
}

const WIDTH = 580;
const HEIGHT = 280;
const NODE_R = 18;
const TOPIC_R = 10;

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
    // Attraction
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
      const r = n.type === 'node' ? NODE_R : TOPIC_R;
      n.x = Math.max(r + 4, Math.min(WIDTH - r - 4, n.x + n.vx));
      n.y = Math.max(r + 4, Math.min(HEIGHT - r - 4, n.y + n.vy));
    }
  }
  return ns;
}

export function NodeGraphViz({ rows }: VizProps) {
  const [nodes, setNodes] = useState<GraphNode[]>([]);
  const [edges, setEdges] = useState<GraphEdge[]>([]);

  useEffect(() => {
    if (rows.length === 0) return;

    const ros2Nodes = new Set<string>();
    const topics = new Set<string>();
    const edgeList: GraphEdge[] = [];

    for (const row of rows) {
      const src = String(row['source_node'] ?? '');
      const tgt = String(row['target_node'] ?? '');
      const topic = String(row['topic'] ?? '');
      if (src) ros2Nodes.add(src);
      if (tgt) ros2Nodes.add(tgt);
      if (topic) topics.add(topic);
      // Edges: source_node → topic, topic → target_node
      if (src && topic) edgeList.push({ source: src, target: topic });
      if (topic && tgt) edgeList.push({ source: topic, target: tgt });
    }

    const all: GraphNode[] = [
      ...[...ros2Nodes].map((id, i, arr) => {
        const angle = (i / arr.length) * 2 * Math.PI;
        const r = 90;
        return { id, type: 'node' as const, x: WIDTH / 2 + r * Math.cos(angle), y: HEIGHT / 2 + r * Math.sin(angle), vx: 0, vy: 0 };
      }),
      ...[...topics].map((id, i, arr) => {
        const angle = (i / arr.length) * 2 * Math.PI + Math.PI / arr.length;
        const r = 50;
        return { id, type: 'topic' as const, x: WIDTH / 2 + r * Math.cos(angle), y: HEIGHT / 2 + r * Math.sin(angle), vx: 0, vy: 0 };
      }),
    ];

    const laid = simpleForce(all, edgeList, 120);
    setNodes(laid);
    setEdges(edgeList);
  }, [rows]);

  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  const nodeMap = new Map<string, GraphNode>(nodes.map((n) => [n.id, n]));

  return (
    <div style={{ overflowX: 'auto' }}>
      <svg width={WIDTH} height={HEIGHT} style={{ display: 'block' }}>
        <defs>
          <marker id="ng-arrow" markerWidth="7" markerHeight="7" refX="6" refY="3" orient="auto">
            <path d="M0,0 L0,6 L7,3 z" fill="#60A5FA" />
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
          const sR = s.type === 'node' ? NODE_R : TOPIC_R;
          const tR = t.type === 'node' ? NODE_R : TOPIC_R;
          return (
            <line
              key={i}
              x1={s.x + (dx / dist) * sR}
              y1={s.y + (dy / dist) * sR}
              x2={t.x - (dx / dist) * (tR + 6)}
              y2={t.y - (dy / dist) * (tR + 6)}
              stroke="#60A5FA"
              strokeWidth={1.2}
              strokeOpacity={0.5}
              markerEnd="url(#ng-arrow)"
            />
          );
        })}

        {/* Nodes */}
        {nodes.map((n) => {
          const isTopic = n.type === 'topic';
          const r = isTopic ? TOPIC_R : NODE_R;
          const fill = isTopic ? '#1E3A5F' : '#1F2937';
          const stroke = isTopic ? '#60A5FA' : '#6EE7B7';
          const shortName = n.id.split('/').pop() ?? n.id;
          return (
            <g key={n.id}>
              {isTopic ? (
                <rect
                  x={n.x - r}
                  y={n.y - r * 0.7}
                  width={r * 2}
                  height={r * 1.4}
                  rx={3}
                  fill={fill}
                  stroke={stroke}
                  strokeWidth={1.5}
                />
              ) : (
                <circle cx={n.x} cy={n.y} r={r} fill={fill} stroke={stroke} strokeWidth={1.5} />
              )}
              <text
                x={n.x}
                y={n.y + 1}
                textAnchor="middle"
                dominantBaseline="middle"
                fill={isTopic ? '#93C5FD' : '#D1D5DB'}
                fontSize={8}
                fontFamily="var(--ifm-font-family-monospace)"
              >
                {shortName.length > 10 ? shortName.slice(0, 10) + '…' : shortName}
              </text>
            </g>
          );
        })}

        {/* Legend */}
        <g transform="translate(8, 8)">
          <circle cx={8} cy={8} r={6} fill="#1F2937" stroke="#6EE7B7" strokeWidth={1.5} />
          <text x={17} y={12} fill="#6B7280" fontSize={8} fontFamily="var(--ifm-font-family-monospace)">node</text>
          <rect x={2} y={22} width={12} height={8} rx={2} fill="#1E3A5F" stroke="#60A5FA" strokeWidth={1.5} />
          <text x={17} y={30} fill="#6B7280" fontSize={8} fontFamily="var(--ifm-font-family-monospace)">topic</text>
        </g>
      </svg>
    </div>
  );
}
