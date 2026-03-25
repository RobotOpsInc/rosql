import type { ReactNode } from 'react';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import CodeBlock from '@theme/CodeBlock';
import Head from '@docusaurus/Head';
import { Bot, ChartScatter, Table, Share2 } from 'lucide-react';
import { RosqlRepl } from '@site/src/components/RosqlRepl';

const JSON_LD = JSON.stringify({
  '@context': 'https://schema.org',
  '@type': 'SoftwareApplication',
  name: 'ROSQL',
  description: 'Open source SQL-like query language purpose-built for ROS2 telemetry data — traces, logs, and metrics with first-class support for nodes, actions, topics, and message causality.',
  url: 'https://rosql.org',
  applicationCategory: 'DeveloperApplication',
  operatingSystem: 'Linux, macOS, Windows',
  offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
  license: 'https://www.apache.org/licenses/LICENSE-2.0',
  codeRepository: 'https://github.com/RobotOpsInc/rosql',
  programmingLanguage: 'Rust',
  keywords: 'ROS2, robotics, telemetry, query language, OpenTelemetry, SQL, observability',
});

const DURING_QUERY = `SELECT trace_id, span_name_col, service_name, duration, status_code, span_attributes
FROM traces
WHERE status = 'ERROR' AND action_name = '/navigate_to_pose'
DURING(
  FROM topics WHERE topic_name = '/battery_state'
  AND fields['percentage'] < 15
)
SINCE 6 hours ago`;

const ARCH_DIAGRAM = `  ROS2 System
       │
       │  ros.node, ros.action.*, ros.topic
       │  ParentSpanId  (OTel attributes)
       ▼
  Robot Ops Agent  or  OTel Collector
       │
       │  OTLP gRPC
       ▼
  Datastore (PostgreSQL, MySQL, DuckDB …)
       │
       │  OTel standard schema
       ▼
  rosql  (parse + compile + execute)
       │
       ▼
  Query results`;

type DriverStatus = 'available' | 'coming-soon' | 'planned';

const DRIVERS: { name: string; flag: string; status: DriverStatus; issue?: string }[] = [
  { name: 'PostgreSQL / TimescaleDB', flag: 'postgres', status: 'available' },
  { name: 'MySQL / MariaDB', flag: 'mysql', status: 'available' },
  { name: 'DuckDB (embedded)', flag: 'duckdb', status: 'coming-soon', issue: '18' },
  { name: 'AWS Athena', flag: 'athena', status: 'planned', issue: '9' },
  { name: 'Google BigQuery', flag: 'bigquery', status: 'planned', issue: '10' },
];

function StatusBadge({ status, issue }: { status: DriverStatus; issue?: string }) {
  const styles: Record<DriverStatus, React.CSSProperties> = {
    available: { background: '#dcfce7', color: '#166534', border: '1px solid #bbf7d0' },
    'coming-soon': { background: '#fef9c3', color: '#854d0e', border: '1px solid #fef08a' },
    planned: { background: '#f1f5f9', color: '#475569', border: '1px solid #cbd5e1' },
  };
  const labels: Record<DriverStatus, string> = {
    available: '✅ v0.1',
    'coming-soon': '🔜 Coming soon',
    planned: '📋 Planned',
  };
  const badge = (
    <span style={{ ...styles[status], borderRadius: 4, padding: '2px 8px', fontSize: 12, whiteSpace: 'nowrap' }}>
      {labels[status]}
    </span>
  );
  if (issue && status !== 'available') {
    return (
      <a href={`https://github.com/RobotOpsInc/rosql/issues/${issue}`} style={{ textDecoration: 'none' }}>
        {badge}
      </a>
    );
  }
  return badge;
}

export default function Home(): ReactNode {
  const { siteConfig } = useDocusaurusContext();

  return (
    <Layout
      title="The query language for robotics telemetry"
      description="ROSQL is an open source SQL-like query language purpose-built for ROS2 telemetry data. Query traces, logs, and metrics with first-class support for nodes, actions, topics, and message causality."
    >
      <Head>
        <script type="application/ld+json">{JSON_LD}</script>
      </Head>
      {/* Hero */}
      <header className="rosql-hero">
        <div className="container" style={{ textAlign: 'center' }}>
          {/* Level 1: name */}
          <h1 className="hero__title" style={{ fontSize: '3.5rem', fontWeight: 800, color: '#F3F4F6', marginBottom: '1rem' }}>
            ROSQL<sup style={{ fontSize: '0.45em', verticalAlign: 'super' }}>™</sup>
          </h1>

          {/* Level 2: tagline — tightly coupled to the name */}
          <p style={{ fontSize: '1.35rem', fontWeight: 500, color: '#D1D5DB', maxWidth: 700, margin: '0 auto 0.5rem', lineHeight: 1.4 }}>
            {siteConfig.tagline}
          </p>

          {/* Level 3: one-liner description — slightly more muted, indented away from tagline */}
          <p style={{ fontSize: '1rem', color: '#9CA3AF', maxWidth: 600, margin: '0 auto 3rem', lineHeight: 1.7 }}>
            A SQL-like language purpose-built for ROS2 telemetry — query traces, logs, and metrics
            with first-class support for nodes, actions, topics, and message causality, stored via{' '}
            <a href="https://opentelemetry.io/" style={{ color: 'var(--ifm-color-primary-lighter)' }}>OpenTelemetry</a>.
          </p>

          {/* Level 4: CTAs — visually separate group */}
          <div className="rosql-hero-buttons">
            <Link className="button button--primary button--lg" to="/docs/quickstart">
              Get started
            </Link>
            <Link className="button button--secondary button--lg" to="/playground">
              Try it live
            </Link>
          </div>

          {/* Level 5: REPL demo */}
          <div style={{ maxWidth: 900, margin: '2rem auto 0', textAlign: 'left' }}>
            <p style={{ textAlign: 'center', fontSize: '0.85rem', color: '#6B7280', marginBottom: '0.75rem', letterSpacing: '0.05em', textTransform: 'uppercase', fontWeight: 500 }}>
              <span style={{ color: 'var(--ifm-color-primary-lighter)' }}>Try it</span> — pick a query and hit Parse or Validate
            </p>
            <RosqlRepl compact />
            <p style={{ textAlign: 'center', fontSize: '0.9rem', color: '#6B7280', marginTop: '1.25rem', maxWidth: 680, margin: '1.25rem auto 0', lineHeight: 1.6 }}>
              Results are unified across signal types — tables for SQL consumers, structured objects
              for programmatic processing, chart-ready data for visualization, and causality graphs
              for message tracing. Available as a library, CLI, gRPC server, and{' '}
              <a href="/docs/wasm" style={{ color: 'var(--ifm-color-primary-lighter)' }}>WASM package</a>.
            </p>
          </div>
        </div>
      </header>

      <main>
        {/* Benefits */}
        <section style={{ padding: '3rem 0', borderBottom: '1px solid var(--ifm-color-emphasis-200)' }}>
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '2rem' }}>
              Why ROSQL?
            </h2>
            <p style={{ textAlign: 'center', maxWidth: 640, margin: '0 auto 2.5rem', color: 'var(--ifm-color-emphasis-700)' }}>
              Robot observability is hard. ROS2 systems generate a firehose of traces, logs, and sensor
              data across dozens of nodes, but general-purpose query languages have no awareness of topics,
              action graphs, or message causality. ROSQL closes that gap: write queries in the language of your robot, <em>not your database.</em>
            </p>
            <div className="benefit-grid">
              <div className="benefit-card">
                <Bot size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Robotics-native syntax</h3>
                <p>First-class support for ROS2 nodes, actions, topics, and <code>ParentSpanId</code>-based message causality. No glue code.</p>
              </div>
              <div className="benefit-card">
                <ChartScatter size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Cross-signal correlation</h3>
                <p><code>DURING()</code> correlates events across traces, logs, metrics, and topics in a single query — something SQL has no primitive for.</p>
              </div>
              <div className="benefit-card">
                <Table size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Unified results</h3>
                <p>Results are returned as structured objects ready for tables, charts, further programmatic processing, or graph visualization.</p>
              </div>
              <div className="benefit-card">
                <Share2 size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Causality graphs</h3>
                <p><code>MESSAGE JOURNEY</code> walks the <code>parent_span_id</code> chain recursively — see exactly how a message propagated through your robot's nodes.</p>
              </div>
            </div>
          </div>
        </section>

        {/* Demo query */}
        <section style={{ padding: '3rem 0', borderBottom: '1px solid var(--ifm-color-emphasis-200)' }}>
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '0.5rem' }}>
              One query, multiple signals
            </h2>
            <p style={{ textAlign: 'center', color: 'var(--ifm-color-emphasis-700)', marginBottom: '2rem' }}>
              Find every navigation failure that happened while the battery was critically low.
              One sentence. One query. No JOINs.
            </p>
            <div style={{ maxWidth: 720, margin: '0 auto' }}>
              <CodeBlock language="sql" title="Cross-signal correlation">
                {DURING_QUERY}
              </CodeBlock>
            </div>
          </div>
        </section>

        {/* Architecture */}
        <section style={{ padding: '3rem 0', borderBottom: '1px solid var(--ifm-color-emphasis-200)' }}>
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '2rem' }}>Architecture</h2>
            <div style={{ maxWidth: 560, margin: '0 auto' }}>
              <div className="arch-diagram">{ARCH_DIAGRAM}</div>
            </div>
            <div style={{ display: 'flex', justifyContent: 'center', gap: '2rem', marginTop: '2rem', flexWrap: 'wrap', textAlign: 'center' }}>
              {[
                { title: 'Library', desc: 'Embed in Rust apps. Parse + execute against any driver.' },
                { title: 'CLI + gRPC', desc: 'rosql query / compile / parse — pipe into scripts.' },
                { title: 'WASM', desc: 'Parse and validate in the browser. No server needed.' },
              ].map(({ title, desc }) => (
                <div key={title} style={{ maxWidth: 220 }}>
                  <strong style={{ display: 'block', marginBottom: 4 }}>{title}</strong>
                  <span style={{ fontSize: '0.9rem', color: 'var(--ifm-color-emphasis-600)' }}>{desc}</span>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* Driver support */}
        <section style={{ padding: '3rem 0', borderBottom: '1px solid var(--ifm-color-emphasis-200)' }}>
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '2rem' }}>Driver support</h2>
            <div style={{ maxWidth: 600, margin: '0 auto' }}>
              <table className="driver-table">
                <thead>
                  <tr>
                    <th>Backend</th>
                    <th>Feature flag</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  {DRIVERS.map(({ name, flag, status, issue }) => (
                    <tr key={flag}>
                      <td>{name}</td>
                      <td><code>{flag}</code></td>
                      <td><StatusBadge status={status} issue={issue} /></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <p style={{ textAlign: 'center', marginTop: '1rem' }}>
              <Link to="/docs/drivers">Driver docs →</Link>
            </p>
          </div>
        </section>

        {/* Quick start */}
        <section style={{ padding: '3rem 0', borderBottom: '1px solid var(--ifm-color-emphasis-200)' }}>
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '0.5rem' }}>Quick start</h2>
            <p style={{ textAlign: 'center', color: 'var(--ifm-color-emphasis-600)', marginBottom: '2rem', fontSize: '0.9rem' }}>
              Requires the{' '}
              <a href="https://rust-lang.org/tools/install/">Rust toolchain</a>.
            </p>
            <div style={{ maxWidth: 720, margin: '0 auto' }}>
              <CodeBlock language="bash" title="Install the CLI">
                {`cargo install rosql --features server,postgres`}
              </CodeBlock>
              <CodeBlock language="bash" title="Run your first query">
                {`rosql query "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago" \\
  --backend postgres \\
  --url postgresql://user:pass@localhost:5432/telemetry`}
              </CodeBlock>
            </div>
            <div style={{ display: 'flex', justifyContent: 'center', gap: 12, marginTop: '1.5rem', flexWrap: 'wrap' }}>
              <Link className="button button--primary" to="/docs/quickstart">Full quickstart →</Link>
              <Link className="button button--secondary" to="/docs/wasm">WASM / Browser →</Link>
            </div>
          </div>
        </section>

        {/* Robot Ops CTA */}
        <section style={{ padding: '3rem 0', background: 'var(--ifm-color-emphasis-100)' }}>
          <div className="container" style={{ textAlign: 'center' }}>
            <h2 style={{ marginBottom: '0.75rem' }}>Need fleet-scale telemetry?</h2>
            <p style={{ maxWidth: 560, margin: '0 auto 1.5rem', color: 'var(--ifm-color-emphasis-700)' }}>
              ROSQL is created and used by <strong>Robot Ops, Inc.</strong> to power the Robot Ops observability platform —
              managed ingestion, storage, and dashboards with lifecycle anchors, fleet-wide anomaly detection,
              and ClickHouse performance.
            </p>
            <a className="button button--primary button--lg" href="https://robotops.com">
              Explore Robot Ops →
            </a>
          </div>
        </section>
      </main>
    </Layout>
  );
}
