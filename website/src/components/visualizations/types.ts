export interface VisualizationConfig {
  x_axis?: string;
  y_axis?: string;
  series_key?: string;
  color_field?: string;
  label_field?: string;
}

export interface VizProps {
  rows: Record<string, unknown>[];
  visualization?: VisualizationConfig;
}
