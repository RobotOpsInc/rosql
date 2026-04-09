import type { ReactNode } from 'react';
import Layout from '@theme/Layout';
import CodeBlock from '@theme/CodeBlock';

type Example = {
  title: string;
  description: string;
  query: string;
  note?: string;
};

const EXAMPLES: Example[] = [
  {
    title: 'Find the navigation action that failed',
    description:
      'Start any investigation by finding which action failed. Filter the traces data source by status and action name.',
    query: "FROM traces WHERE status = 'ERROR' AND action_name = '/navigate_to_pose'",
  },
  {
    title: 'Cross-signal correlation: failures during low battery',
    description:
      'The DURING() clause correlates two data sources by time. Here we find navigation failures that occurred while battery percentage was critically low — combining a trace query with a topic message query in a single statement.',
    query: `SELECT trace_id, span_name_col, service_name, duration, status_code, span_attributes
FROM traces
WHERE status = 'ERROR' AND action_name = '/navigate_to_pose'
DURING(
  FROM topics WHERE topic_name = '/battery_state'
  AND fields['percentage'] < 15
)
SINCE 6 hours ago`,
    note: 'DURING() is ROSQL\'s most powerful feature. It replaces complex multi-table JOINs with a single, readable clause.',
  },
  {
    title: 'Trace the full message causality chain',
    description:
      'MESSAGE JOURNEY walks parent_span_id → span_id recursively and returns every span in the causality tree. This reveals exactly which nodes were involved and in what order — something plain SQL has no primitive for.',
    query: "MESSAGE JOURNEY FOR TRACE 'a3f1c9d2e8b04f7a'",
    note: 'Requires ParentSpanId to be set correctly in your OTel instrumentation. See Schema Reference for details.',
  },
  {
    title: 'Find all message paths for a topic',
    description:
      'MESSAGE PATHS reveals which nodes published and subscribed to a given topic. Useful for understanding your robot\'s communication graph.',
    query: "MESSAGE PATHS FOR TOPIC '/cmd_vel'",
  },
  {
    title: 'Error rate by robot (health dashboard)',
    description:
      'Get a real-time error rate breakdown per robot. One of five composable health queries that together replace the upcoming HEALTH() command.',
    query: `SELECT COUNT(*) FROM traces WHERE status = 'ERROR' FACET robot_id SINCE 30 minutes ago`,
  },
  {
    title: 'Action success rate',
    description:
      'ACTION_SUCCESS_RATE() computes the fraction of succeeded action spans to total spans. Returns a value between 0 and 1 — useful for SLO tracking and alerting.',
    query: "SELECT ACTION_SUCCESS_RATE('/navigate_to_pose') FROM traces SINCE 1 hour ago",
  },
  {
    title: 'Topic publish rate',
    description:
      'TOPIC_RATE() queries the otel_metrics table for ros2.topic.message_rate values. Pass a topic name to filter to a specific topic.',
    query: "SELECT TOPIC_RATE('/cmd_vel') FROM metrics SINCE 30 minutes ago",
  },
  {
    title: 'Rolling average latency (MOVING_AVG)',
    description:
      'MOVING_AVG smooths out per-span latency spikes using a sliding window. Compiles to a SQL window function — no post-processing needed.',
    query: 'SELECT MOVING_AVG(duration, 5) FROM traces WHERE action_name = \'/navigate_to_pose\'',
  },
  {
    title: 'Pipeline syntax: slow errors grouped by robot',
    description:
      'Pipeline syntax uses | to chain stages, making multi-step queries readable at a glance. Each stage filters or transforms the output of the previous one.',
    query: `FROM traces
| WHERE duration > 500 ms
| WHERE status = 'ERROR'
| FACET robot_id`,
  },
  {
    title: 'Compare navigation failures to last week',
    description:
      'COMPARE TO shows current and baseline counts side by side. Instantly see if error rates are trending up or down relative to a historical baseline.',
    query: `FROM traces
| WHERE action_name = '/navigate_to_pose'
| WHERE status = 'ERROR'
| FACET robot_id
| COMPARE TO last week`,
  },
];

export default function Examples(): ReactNode {
  return (
    <Layout
      title="Examples"
      description="ROSQL query examples — cross-signal correlation, message causality, health assessment, anomaly detection, and more"
    >
      <div className="container" style={{ padding: '2.5rem 0 4rem' }}>
        <div style={{ maxWidth: 800, margin: '0 auto' }}>
          <h1>Examples</h1>
          <p style={{ fontSize: '1.1rem', color: 'var(--ifm-color-emphasis-700)', marginBottom: '3rem' }}>
            Query examples from trivial to advanced. These showcase ROSQL's unique robotics-native features —
            things that would take pages of SQL, ROSQL expresses in a single line.
          </p>

          {EXAMPLES.map(({ title, description, query, note }, i) => (
            <div key={i} style={{ marginBottom: '3rem', paddingBottom: '3rem', borderBottom: i < EXAMPLES.length - 1 ? '1px solid var(--ifm-color-emphasis-200)' : 'none' }}>
              <h2 style={{ fontSize: '1.3rem', marginBottom: '0.5rem' }}>{title}</h2>
              <p style={{ color: 'var(--ifm-color-emphasis-700)', marginBottom: '1rem' }}>{description}</p>
              <CodeBlock language="sql">{query}</CodeBlock>
              {note && (
                <p style={{ fontSize: '0.875rem', color: 'var(--ifm-color-emphasis-600)', marginTop: '0.5rem', fontStyle: 'italic' }}>
                  💡 {note}
                </p>
              )}
            </div>
          ))}

          <div style={{ textAlign: 'center', paddingTop: '1rem' }}>
            <p style={{ color: 'var(--ifm-color-emphasis-700)', marginBottom: '1rem' }}>
              Ready to try these against your own data?
            </p>
            <a className="button button--primary button--lg" href="/docs/quickstart">
              Quickstart →
            </a>
          </div>
        </div>
      </div>
    </Layout>
  );
}
