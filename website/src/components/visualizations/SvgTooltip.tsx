import React from 'react';

export interface SvgTooltipProps {
  text: string;
  /** SVG x coordinate to anchor the tooltip (centred above this point). */
  cx: number;
  /** SVG y coordinate — tooltip appears above this point. */
  cy: number;
  /** SVG viewport width used to clamp the tooltip horizontally. */
  svgWidth: number;
}

/**
 * An SVG-native tooltip rendered as a dark rounded rect + text.
 * Render this as the last child of your <svg> so it sits above all other elements.
 * Set `pointerEvents="none"` on the group so it never blocks hover events.
 */
export function SvgTooltip({ text, cx, cy, svgWidth }: SvgTooltipProps) {
  const CHAR_W = 6.5;
  const PADDING = 8;
  const BOX_H = 20;
  const boxW = Math.min(text.length * CHAR_W + PADDING * 2, svgWidth - 8);
  const clampedCx = Math.max(boxW / 2 + 4, Math.min(svgWidth - boxW / 2 - 4, cx));
  const clampedCy = Math.max(BOX_H + 4, cy);
  return (
    <g pointerEvents="none">
      <rect
        x={clampedCx - boxW / 2}
        y={clampedCy - BOX_H}
        width={boxW}
        height={BOX_H}
        rx={4}
        fill="#111827"
        stroke="#4B5563"
        strokeWidth={1}
      />
      <text
        x={clampedCx}
        y={clampedCy - BOX_H / 2 + 1}
        textAnchor="middle"
        dominantBaseline="middle"
        fill="#F9FAFB"
        fontSize={10}
        fontFamily="var(--ifm-font-family-monospace)"
      >
        {text}
      </text>
    </g>
  );
}
