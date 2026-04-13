import React, { useEffect, useRef, useState } from 'react';
import type { VizProps } from './types';

interface Node {
  id: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
}

interface Edge {
  source: string;
  target: string;
  label: string;
}

const WIDTH = 560;
const HEIGHT = 260;
const NODE_R = 20;

function forceLayout(nodes: Node[], edges: Edge[], iterations = 80): Node[] {
  const ns = nodes.map((n) => ({ ...n }));

  for (let iter = 0; iter < iterations; iter++) {
    // Repulsion
    for (let i = 0; i < ns.length; i++) {
      for (let j = i + 1; j < ns.length; j++) {
        const dx = ns[j].x - ns[i].x;
        const dy = ns[j].y - ns[i].y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        const force = 8000 / (dist * dist);
        ns[i].vx -= (dx / dist) * force;
        ns[i].vy -= (dy / dist) * force;
        ns[j].vx += (dx / dist) * force;
        ns[j].vy += (dy / dist) * force;
      }
    }
    // Attraction along edges
    for (const edge of edges) {
      const s = ns.find((n) => n.id === edge.source);
      const t = ns.find((n) => n.id === edge.target);
      if (!s || !t) continue;
      const dx = t.x - s.x;
      const dy = t.y - s.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      const force = dist / 80;
      s.vx += (dx / dist) * force;
      s.vy += (dy / dist) * force;
      t.vx -= (dx / dist) * force;
      t.vy -= (dy / dist) * force;
    }
    // Center gravity
    for (const n of ns) {
      n.vx += (WIDTH / 2 - n.x) * 0.005;
      n.vy += (HEIGHT / 2 - n.y) * 0.005;
    }
    // Apply with damping and clamp
    const damping = 0.85;
    for (const n of ns) {
      n.vx *= damping;
      n.vy *= damping;
      n.x = Math.max(NODE_R + 4, Math.min(WIDTH - NODE_R - 4, n.x + n.vx));
      n.y = Math.max(NODE_R + 4, Math.min(HEIGHT - NODE_R - 4, n.y + n.vy));
    }
  }
  return ns;
}

export function DirectedGraphViz({ rows }: VizProps) {
  const [nodes, setNodes] = useState<Node[]>([]);
  const [edges, setEdges] = useState<Edge[]>([]);

  useEffect(() => {
    if (rows.length === 0) return;

    const nodeIds = new Set<string>();
    const edgeList: Edge[] = [];

    // Each row: source_node, topic/label, target_node (or span hierarchy)
    for (const row of rows) {
      const src = String(row['source_node'] ?? row['span_name_col'] ?? row['service_name'] ?? '');
      const tgt = String(row['target_node'] ?? row['parent_span_id'] ?? '');
      const label = String(row['topic'] ?? row['span_name_col'] ?? '');
      if (src) nodeIds.add(src);
      if (tgt) nodeIds.add(tgt);
      if (src && tgt) edgeList.push({ source: src, target: tgt, label });
    }

    // Initialize nodes in a circle
    const nodeArr = [...nodeIds].map((id, i, arr) => {
      const angle = (i / arr.length) * 2 * Math.PI;
      const r = Math.min(WIDTH, HEIGHT) / 2 - NODE_R - 20;
      return {
        id,
        x: WIDTH / 2 + r * Math.cos(angle),
        y: HEIGHT / 2 + r * Math.sin(angle),
        vx: 0,
        vy: 0,
      };
    });

    const laid = forceLayout(nodeArr, edgeList, 100);
    setNodes(laid);
    setEdges(edgeList);
  }, [rows]);

  if (rows.length === 0) {
    return <div style={{ color: '#6B7280', padding: '16px 0', textAlign: 'center' }}>No results</div>;
  }

  function arrowEndpoint(s: Node, t: Node) {
    const dx = t.x - s.x;
    const dy = t.y - s.y;
    const dist = Math.sqrt(dx * dx + dy * dy) || 1;
    return {
      x1: s.x + (dx / dist) * NODE_R,
      y1: s.y + (dy / dist) * NODE_R,
      x2: t.x - (dx / dist) * (NODE_R + 6),
      y2: t.y - (dy / dist) * (NODE_R + 6),
    };
  }

  const nodeMap = new Map<string, Node>(nodes.map((n) => [n.id, n]));

  return (
    <div style={{ overflowX: 'auto' }}>
      <svg width={WIDTH} height={HEIGHT} style={{ display: 'block' }}>
        <defs>
          <marker id="arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
            <path d="M0,0 L0,6 L8,3 z" fill="#6EE7B7" />
          </marker>
        </defs>

        {/* Edges */}
        {edges.map((edge, i) => {
          const s = nodeMap.get(edge.source);
          const t = nodeMap.get(edge.target);
          if (!s || !t) return null;
          const { x1, y1, x2, y2 } = arrowEndpoint(s, t);
          const mx = (x1 + x2) / 2;
          const my = (y1 + y2) / 2;
          return (
            <g key={i}>
              <line
                x1={x1} y1={y1} x2={x2} y2={y2}
                stroke="#6EE7B7"
                strokeWidth={1.5}
                strokeOpacity={0.6}
                markerEnd="url(#arrow)"
              />
              {edge.label && (
                <text x={mx} y={my - 4} textAnchor="middle" fill="#6B7280" fontSize={9} fontFamily="var(--ifm-font-family-monospace)">
                  {edge.label}
                </text>
              )}
            </g>
          );
        })}

        {/* Nodes */}
        {nodes.map((node) => {
          const shortName = node.id.split('/').pop() ?? node.id;
          return (
            <g key={node.id}>
              <circle cx={node.x} cy={node.y} r={NODE_R} fill="#1F2937" stroke="#6EE7B7" strokeWidth={1.5} />
              <text
                x={node.x}
                y={node.y + 1}
                textAnchor="middle"
                dominantBaseline="middle"
                fill="#D1D5DB"
                fontSize={8}
                fontFamily="var(--ifm-font-family-monospace)"
              >
                {shortName.length > 12 ? shortName.slice(0, 12) + '…' : shortName}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}
