import type { ReactNode } from 'react';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import CodeBlock from '@theme/CodeBlock';
import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import Head from '@docusaurus/Head';
import { Bot, ChartScatter, Table, Share2, Terminal, Database, BarChart2, ArrowRight, Ruler, BarChart3, Network, Cpu, Layers, Activity, Box, Play, GitBranch } from 'lucide-react';
import { RosqlRepl } from '@site/src/components/RosqlRepl';

const JSON_LD = JSON.stringify({
  '@context': 'https://schema.org',
  '@type': 'SoftwareApplication',
  name: 'ROSQL',
  description: 'Open source SQL-like query language purpose-built for ROS2 telemetry — distributed tracing, logs, metrics, and pose/kinematics with first-class OpenTelemetry support.',
  url: 'https://rosql.org',
  applicationCategory: 'DeveloperApplication',
  operatingSystem: 'Linux, macOS, Windows',
  offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
  license: 'https://www.apache.org/licenses/LICENSE-2.0',
  codeRepository: 'https://github.com/RobotOpsInc/rosql',
  programmingLanguage: 'Rust',
  keywords: 'ROS2, robotics, telemetry, query language, OpenTelemetry, distributed tracing, SQL, observability, kinematics, pose, Nav2, MoveIt',
});

const DURING_QUERY = `FROM joint_states
WHERE fields['position[2]'] > 30 deg
DURING(
  FROM battery WHERE fields['percentage'] < 15
)
FOR ROBOT 'arm-01'
SINCE last deployment`;

const TIMESERIES_QUERY = `SELECT cpu_usage FROM metrics
TIMESERIES 2 min FACET robot_id
SINCE 45 min ago`;

const ARCH_DIAGRAM = `  ROS2 System
       │
       │  ros.node, ros.action.*, ros.topic
       │  ParentSpanId  (OTel attributes)
       ▼
  Robot Ops Agent  or  OTel Collector
       │
       │  OTLP gRPC
       ▼
  Datastore (PostgreSQL, Clickhouse, Parquet/S3 …)
       │
       │  OTel standard schema
       ▼
  rosql  (parse + compile + execute)
       │
       ▼
  Query results`;

type DriverStatus = 'available' | 'coming-soon' | 'planned';

const DRIVERS: { name: string; flag: string; status: DriverStatus; version?: string; issue?: string }[] = [
  { name: 'PostgreSQL / TimescaleDB', flag: 'postgres', status: 'available', version: 'v0.1' },
  { name: 'MySQL / MariaDB', flag: 'mysql', status: 'available', version: 'v0.1' },
  { name: 'Parquet (local / S3) via DuckDB', flag: 'duckdb', status: 'available', version: 'v0.4.5' },
  { name: 'ClickHouse', flag: 'clickhouse', status: 'planned', issue: '98' },
  { name: 'AWS Athena', flag: 'athena', status: 'planned', issue: '9' },
  { name: 'Google BigQuery', flag: 'bigquery', status: 'planned', issue: '10' },
];

function StatusBadge({ status, version, issue }: { status: DriverStatus; version?: string; issue?: string }) {
  const styles: Record<DriverStatus, React.CSSProperties> = {
    available: { background: '#dcfce7', color: '#166534', border: '1px solid #bbf7d0' },
    'coming-soon': { background: '#fef9c3', color: '#854d0e', border: '1px solid #fef08a' },
    planned: { background: '#f1f5f9', color: '#475569', border: '1px solid #cbd5e1' },
  };
  const labels: Record<DriverStatus, string> = {
    available: `✅ ${version ?? 'v0.1'}`,
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

// ---------------------------------------------------------------------------
// Showcase mockup visualizations
// ---------------------------------------------------------------------------

function MockTable({ rows, highlightRow }: {
  rows: { cols: string[] }[];
  highlightRow?: number;
}) {
  return (
    <div className="mock-table-wrap">
      {rows.map((row, i) => (
        <div key={i} className={`mock-table-row${i === highlightRow ? ' mock-table-row--highlight' : ''}`}>
          {row.cols.map((col, j) => (
            <span key={j} className="mock-table-cell">{col}</span>
          ))}
        </div>
      ))}
    </div>
  );
}

function MockLogTable() {
  const entries = [
    { level: 'ERROR', msg: 'Action /navigate_to_pose failed: goal rejected', trace: 'a1b2…' },
    { level: 'WARN',  msg: 'Costmap inflation radius exceeded threshold',    trace: 'a1b2…' },
    { level: 'INFO',  msg: 'BT node NavigateToPose ticked',                  trace: 'a1b2…' },
    { level: 'ERROR', msg: 'TF lookup timeout: map → base_link',             trace: 'a1b2…' },
  ];
  const colors: Record<string, string> = {
    ERROR: '#fee2e2',
    WARN:  '#fef9c3',
    INFO:  '#e0f2fe',
  };
  const text: Record<string, string> = {
    ERROR: '#991b1b',
    WARN:  '#854d0e',
    INFO:  '#075985',
  };
  return (
    <div className="mock-table-wrap">
      {entries.map((e, i) => (
        <div key={i} className="mock-table-row" style={{ gap: '0.5rem' }}>
          <span style={{ background: colors[e.level], color: text[e.level], borderRadius: 3, padding: '1px 5px', fontSize: '0.65rem', fontWeight: 700, whiteSpace: 'nowrap' }}>{e.level}</span>
          <span className="mock-table-cell" style={{ flex: 1, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{e.msg}</span>
          <span className="mock-table-cell" style={{ color: 'var(--ifm-color-primary-lighter)', fontSize: '0.65rem' }}>{e.trace}</span>
        </div>
      ))}
    </div>
  );
}

function MockGantt() {
  const spans = [
    { name: 'navigate_to_pose',       start: 0,   width: 100, highlight: false },
    { name: '  compute_path',         start: 3,   width: 42,  highlight: false },
    { name: '    planner_server',      start: 5,   width: 38,  highlight: true  },
    { name: '  follow_path',          start: 47,  width: 48,  highlight: false },
    { name: '    controller_server',  start: 49,  width: 22,  highlight: false },
  ];
  return (
    <div className="mock-gantt">
      {spans.map((s, i) => (
        <div key={i} className="mock-gantt-row">
          <span className="mock-gantt-label">{s.name}</span>
          <div className="mock-gantt-track">
            <div
              className={`mock-gantt-bar${s.highlight ? ' mock-gantt-bar--hot' : ''}`}
              style={{ marginLeft: `${s.start}%`, width: `${s.width}%` }}
            />
          </div>
        </div>
      ))}
    </div>
  );
}

function MockLineCharts() {
  // y=5 is top, y=85 is baseline — full vertical spread across the viewBox
  const r123 = '0,75 20,58 40,50 60,44 80,12 100,28 120,36 140,8  160,18';
  const r456 = '0,72 20,66 40,55 60,62 80,58 100,46 120,32 140,22 160,30';
  return (
    <div className="mock-linechart-item" style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={{ display: 'flex', gap: '1rem', marginBottom: 6 }}>
        <span style={{ fontSize: '0.65rem', color: '#E74C3C', display: 'flex', alignItems: 'center', gap: 4 }}>
          <svg width="16" height="4" aria-hidden><line x1="0" y1="2" x2="16" y2="2" stroke="#E74C3C" strokeWidth="2.5"/></svg>
          robot-123
        </span>
        <span style={{ fontSize: '0.65rem', color: '#3B82F6', display: 'flex', alignItems: 'center', gap: 4 }}>
          <svg width="16" height="4" aria-hidden><line x1="0" y1="2" x2="16" y2="2" stroke="#3B82F6" strokeWidth="2.5"/></svg>
          robot-456
        </span>
      </div>
      <svg viewBox="0 0 160 90" className="mock-linechart-svg" style={{ flex: 1, width: '100%', height: 'auto', minHeight: 80 }} aria-hidden>
        <line x1="0" y1="85" x2="160" y2="85" stroke="#6B7280" strokeWidth="0.5"/>
        <polyline points={r456} fill="none" stroke="#3B82F6" strokeWidth="2.5" strokeLinejoin="round" strokeLinecap="round"/>
        <polyline points={r123} fill="none" stroke="#E74C3C" strokeWidth="2.5" strokeLinejoin="round" strokeLinecap="round"/>
      </svg>
    </div>
  );
}

function MockRobotArm() {
  return (
    <div className="mock-robot-arm" style={{ width: '100%' }}>
      <svg viewBox="0 0 200 130" aria-hidden style={{ width: '100%', height: 'auto', display: 'block' }}>
        {/* Base */}
        <rect x="80" y="108" width="40" height="14" rx="3" fill="#374151" stroke="#6B7280" strokeWidth="1"/>
        {/* Link 1 */}
        <line x1="100" y1="108" x2="100" y2="76" stroke="#9CA3AF" strokeWidth="5" strokeLinecap="round"/>
        <circle cx="100" cy="76" r="5" fill="#E74C3C" stroke="#374151" strokeWidth="1.5"/>
        {/* Link 2 */}
        <line x1="100" y1="76" x2="128" y2="53" stroke="#9CA3AF" strokeWidth="4" strokeLinecap="round"/>
        <circle cx="128" cy="53" r="4.5" fill="#E74C3C" stroke="#374151" strokeWidth="1.5"/>
        {/* Link 3 */}
        <line x1="128" y1="53" x2="148" y2="30" stroke="#9CA3AF" strokeWidth="3.5" strokeLinecap="round"/>
        <circle cx="148" cy="30" r="4" fill="#F59E0B" stroke="#374151" strokeWidth="1.5"/>
        {/* End effector */}
        <line x1="148" y1="30" x2="160" y2="16" stroke="#9CA3AF" strokeWidth="2.5" strokeLinecap="round"/>
        <line x1="160" y1="16" x2="168" y2="10" stroke="#9CA3AF" strokeWidth="2" strokeLinecap="round"/>
        <line x1="160" y1="16" x2="154" y2="10" stroke="#9CA3AF" strokeWidth="2" strokeLinecap="round"/>
        {/* Joint labels */}
        <text x="86" y="74" fontSize="8" fill="#6B7280" fontFamily="monospace">J1</text>
        <text x="132" y="51" fontSize="8" fill="#6B7280" fontFamily="monospace">J2</text>
        <text x="152" y="28" fontSize="8" fill="#F59E0B" fontFamily="monospace">J3⚠</text>
      </svg>
      {/* Playback controls */}
      <div className="mock-playback">
        <button className="mock-play-btn" aria-label="Play">
          <Play size={10} fill="currentColor" strokeWidth={0} />
        </button>
        <div className="mock-timeline-track">
          <div className="mock-timeline-fill" />
          <div className="mock-timeline-thumb" />
        </div>
        <span className="mock-timecode">0:12 <span style={{ opacity: 0.4 }}>/ 0:15</span></span>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

// Logos with optional dark/light variants. For themed logos we render both
// and use CSS ([data-theme='dark'] / [data-theme='light']) to toggle visibility.
type DbLogo = { name: string; alt: string; planned?: true } & (
  | { src: string; srcDark?: never; srcLight?: never }
  | { src?: never; srcDark: string; srcLight: string }
);

const DB_LOGOS: DbLogo[] = [
  { name: 'Postgres',  alt: 'PostgreSQL',  src: '/img/logos/postgresql-logo.svg' },
  { name: 'Timescale', alt: 'TimescaleDB', srcDark: '/img/logos/timescale-logo-dark.svg',   srcLight: '/img/logos/timescale-logo-light.svg'   },
  { name: 'ClickHouse',alt: 'ClickHouse',  srcDark: '/img/logos/clickhouse-logo-dark.svg',  srcLight: '/img/logos/clickhouse-logo-light.svg'  },
  { name: 'Parquet',   alt: 'Parquet',     src: '/img/logos/parquet-logo.svg' },
  { name: 'BigQuery',  alt: 'BigQuery',    src: '/img/logos/bigquery-logo.svg', planned: true },
];

export default function Home(): ReactNode {
  const { siteConfig } = useDocusaurusContext();

  return (
    <Layout
      title="The query language for robotics telemetry"
      description="ROSQL is an open source SQL-like query language purpose-built for ROS2 telemetry — distributed tracing, logs, metrics, and pose/kinematics with first-class OpenTelemetry support."
    >
      <Head>
        <script type="application/ld+json">{JSON_LD}</script>
      </Head>

      {/* ------------------------------------------------------------------ */}
      {/* Hero                                                                */}
      {/* ------------------------------------------------------------------ */}
      <header className="rosql-hero">
        <div className="container" style={{ textAlign: 'center' }}>
          <h1 className="hero__title" style={{ fontSize: '3.5rem', fontWeight: 800, color: '#F3F4F6', marginBottom: '1rem' }}>
            ROSQL<sup style={{ fontSize: '0.3em', verticalAlign: 'super' }}>™</sup>
          </h1>

          <p style={{ fontSize: '1.35rem', fontWeight: 500, color: '#D1D5DB', maxWidth: 700, margin: '0 auto 0.5rem', lineHeight: 1.4 }}>
            {siteConfig.tagline}
          </p>

          <p style={{ fontSize: '1rem', color: '#9CA3AF', maxWidth: 620, margin: '0 auto 3rem', lineHeight: 1.7 }}>
            A SQL-like language purpose-built for ROS2 telemetry — distributed tracing, logs, metrics,
            and pose/kinematics with first-class support for nodes, actions, topics, and message causality,
            powered by{' '}
            <a href="https://opentelemetry.io/" style={{ color: 'var(--ifm-color-primary-lighter)' }}>OpenTelemetry</a>.
          </p>

          <div className="rosql-hero-buttons">
            <Link className="button button--primary button--lg" to="/docs/quickstart">
              Get started
            </Link>
            <Link className="button button--secondary button--lg" to="/playground">
              Try it live
            </Link>
          </div>

          <div style={{ maxWidth: 900, margin: '2rem auto 0', textAlign: 'left' }}>
            <p style={{ textAlign: 'center', fontSize: '0.85rem', color: '#6B7280', marginBottom: '0.75rem', letterSpacing: '0.05em', textTransform: 'uppercase', fontWeight: 500 }}>
              <span style={{ color: 'var(--ifm-color-primary-lighter)' }}>Try it</span> — pick a scenario and hit Run
            </p>
            <RosqlRepl compact />
            <p style={{ textAlign: 'center', fontSize: '0.8rem', color: '#6B7280', marginTop: '0.75rem' }}>
              Querying a 3-robot warehouse fleet ·{' '}
              <a href="/docs/repl-dataset" style={{ color: 'var(--ifm-color-primary-lighter)' }}>About this dataset →</a>
            </p>
            <p style={{ textAlign: 'center', fontSize: '0.85rem', color: '#4B5563', marginTop: '1rem' }}>
              Available as a library, CLI, gRPC server, and{' '}
              <a href="/docs/wasm" style={{ color: 'var(--ifm-color-primary-lighter)' }}>WASM package</a>.
            </p>
          </div>
        </div>
      </header>

      <main>
        {/* ---------------------------------------------------------------- */}
        {/* Flow diagram                                                      */}
        {/* ---------------------------------------------------------------- */}
        <section className="unified-output-section">
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '0.5rem' }}>One query. Every format you need.</h2>
            <p style={{ textAlign: 'center', maxWidth: 580, margin: '0 auto 3rem', color: 'var(--ifm-color-emphasis-700)' }}>
              Results are unified across signal types — tables for SQL consumers, structured objects
              for programmatic processing, chart-ready data for visualization, and causality graphs
              for message tracing.
            </p>
            <div className="unified-flow">
              {/* ROSQL source */}
              <div className="unified-flow-node unified-flow-node--source">
                <Terminal size={26} strokeWidth={1.5} className="unified-flow-icon" />
                <span className="unified-flow-label">ROSQL Query</span>
                <span className="unified-flow-sub">Your robot's language</span>
              </div>

              <ArrowRight size={20} className="unified-flow-arrow" />

              {/* Telemetry database — logo bar */}
              <div className="unified-flow-node unified-flow-node--db">
                <Database size={26} strokeWidth={1.5} className="unified-flow-icon" />
                <span className="unified-flow-label">Robot Telemetry</span>
                <div className="db-logo-bar">
                  {DB_LOGOS.map((logo) => (
                    <div key={logo.name} className={`db-logo-item${logo.planned ? ' db-logo-item--planned' : ''}`}>
                      {logo.src ? (
                        <img src={logo.src} alt={logo.alt} className="db-logo-img" />
                      ) : (
                        <>
                          <img src={logo.srcDark}  alt={logo.alt} className="db-logo-img db-logo-img--dark" />
                          <img src={logo.srcLight} alt={logo.alt} className="db-logo-img db-logo-img--light" />
                        </>
                      )}
                      <span className="db-logo-name">{logo.name}</span>
                      {logo.planned && <span className="db-logo-badge">soon</span>}
                    </div>
                  ))}
                </div>
              </div>

              <ArrowRight size={20} className="unified-flow-arrow" />

              {/* Outputs */}
              <div className="unified-flow-outputs">
                {([
                  { Icon: Table,     label: 'Structured data',      desc: 'Tables, JSON objects, programmatic output' },
                  { Icon: BarChart2, label: 'Time-series & charts',  desc: 'Fleet metrics via TIMESERIES + FACET' },
                  { Icon: Activity,  label: 'Sensor & topic data',   desc: 'Unit-aware readings, topic streams' },
                  { Icon: BarChart3, label: 'Causality & traces',    desc: 'Gantt spans, topology graphs, directed graphs' },
                  { Icon: Box,       label: '3D joint replay',       desc: 'Joint state animations from recorded data' },
                ] as const).map(({ Icon, label, desc }) => (
                  <div key={label} className="unified-output-card">
                    <Icon size={16} strokeWidth={1.5} className="unified-output-icon" />
                    <div>
                      <strong className="unified-output-label">{label}</strong>
                      <span className="unified-output-desc">{desc}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </section>

        {/* ---------------------------------------------------------------- */}
        {/* Benefits                                                          */}
        {/* ---------------------------------------------------------------- */}
        <section style={{ padding: '3rem 0', borderBottom: '1px solid var(--ifm-color-emphasis-200)' }}>
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '2rem' }}>Why ROSQL?</h2>
            <p style={{ textAlign: 'center', maxWidth: 640, margin: '0 auto 2.5rem', color: 'var(--ifm-color-emphasis-700)' }}>
              Robot observability is hard. ROS2 systems generate a firehose of traces, logs, and sensor
              data across dozens of nodes, but general-purpose query languages have no awareness of topics,
              action graphs, or message causality. ROSQL closes that gap: write queries in the language of your robot, <em>not your database.</em>
            </p>
            <div className="benefit-grid">

              <div className="benefit-card">
                <Bot size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Robotics-native syntax</h3>
                <p>First-class support for ROS2 nodes, actions, topics, named joints, and <code>ParentSpanId</code>-based message causality. No glue code.</p>
              </div>

              <div className="benefit-card">
                <Network size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Distributed tracing</h3>
                <p>Full OpenTelemetry-native tracing — not just within a robot's nodes, but across robots, fleet management systems, and external services. Trace a mission from dispatch to completion.</p>
              </div>

              <div className="benefit-card">
                <ChartScatter size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Cross-signal correlation</h3>
                <p><code>DURING()</code> correlates events across traces, logs, metrics, and topics in a single query — something SQL has no primitive for.</p>
              </div>

              <div className="benefit-card">
                <Layers size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Native framework support</h3>
                <p>First-class topic aliases for common ROS2 frameworks: <code>FROM odom</code>, <code>FROM joint_states</code>, <code>FROM cmd_vel</code>, <code>FROM imu</code>. Nav2 and MoveIt data without memorizing topic names.</p>
              </div>

              <div className="benefit-card">
                <Box size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Robot model understanding</h3>
                <p>Query by named joint, inspect URDF-derived limits with <code>SHOW JOINTS</code>, and measure actual-vs-planned deviation with <code>JOINT DEVIATION</code> and <code>PATH DEVIATION</code>.</p>
              </div>

              <div className="benefit-card">
                <Cpu size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Deep systems telemetry</h3>
                <p>Not just ROS2 — query CPU, memory, network, and custom metrics alongside robot data. Full OpenTelemetry compatibility means any OTel-instrumented service is a first-class citizen.</p>
              </div>

              <div className="benefit-card">
                <GitBranch size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Deployment-aware scoping</h3>
                <p>Scope any query to a specific robot, version, environment, or session — <code>FOR ROBOT 'arm-01' FOR VERSION 'v1.3.0'</code>. Compare error rates across versions or flag regressions since your last deploy without timestamp math.</p>
              </div>

              <div className="benefit-card">
                <Share2 size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Causality graphs</h3>
                <p><code>TRACE 'id'</code> walks the <code>parent_span_id</code> chain recursively — see exactly how a message propagated through your robot's nodes and across system boundaries.</p>
              </div>

              <div className="benefit-card">
                <Ruler size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Unit-aware filtering</h3>
                <p>Write physical quantities in any unit — <code>fields['joint_angle'] &gt; 30 deg</code>, <code>position WITHIN 500 m OF (lat, lon)</code>, <code>fields['torque'] &gt; 12 Nm</code>. ROSQL auto-converts to SI for geospatial, kinematic, and sensor queries alike. No manual conversion.</p>
              </div>

              <div className="benefit-card">
                <BarChart3 size={22} strokeWidth={1.5} style={{ color: 'var(--ifm-color-primary)', marginBottom: '0.75rem' }} />
                <h3>Visual format hints</h3>
                <p>Every query returns a <code>format_hint</code> — Gantt, StackedLineChart, NodeGraph, and more — so your UI renders the right visualization automatically.</p>
              </div>

            </div>
          </div>
        </section>

        {/* ---------------------------------------------------------------- */}
        {/* Showcase                                                          */}
        {/* ---------------------------------------------------------------- */}
        <section style={{ padding: '3rem 0', borderBottom: '1px solid var(--ifm-color-emphasis-200)' }}>
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '0.5rem' }}>Complex questions, instant answers</h2>
            <p style={{ textAlign: 'center', maxWidth: 560, margin: '0 auto 3rem', color: 'var(--ifm-color-emphasis-700)' }}>
              From proximity checks to 3D joint replay — ask in plain English, query in ROSQL.
            </p>
            <div className="showcase-grid">

              <div className="showcase-card">
                <p className="showcase-question">"Did robot <em>amr-foo-1</em> move within 2 ft of a point in the last 24 hours?"</p>
                <div className="showcase-viz">
                  <MockTable
                    rows={[
                      { cols: ['09:23:14', 'amr-foo-1', '29.512 ft', ''] },
                      { cols: ['09:24:02', 'amr-foo-1', '26.342 ft', ''] },
                      { cols: ['09:24:51', 'amr-foo-1', '23.143 ft', '✓'] },
                      { cols: ['09:25:37', 'amr-foo-1', '21.188 ft', ''] },
                    ]}
                  />
                </div>
              </div>

              <div className="showcase-card">
                <p className="showcase-question">"Did the link <em>left_shoulder</em> ever exceed 30° since the last deploy?"</p>
                <div className="showcase-viz">
                  <MockTable
                    rows={[
                      { cols: ['10:15:08', 'left_shoulder', '28.3°', ''] },
                      { cols: ['10:16:22', 'left_shoulder', '31.7°', '⚠'] },
                      { cols: ['10:17:05', 'left_shoulder', '29.1°', ''] },
                    ]}
                    highlightRow={1}
                  />
                </div>
              </div>

              <div className="showcase-card">
                <p className="showcase-question">"What was the joint deviation on <em>robot-123</em> vs <em>robot-456</em> in the last hour?"</p>
                <div className="showcase-viz">
                  <MockLineCharts />
                </div>
              </div>

              <div className="showcase-card">
                <p className="showcase-question">"Show me the 3D joint states in the 15 seconds before the error"</p>
                <div className="showcase-viz">
                  <MockRobotArm />
                </div>
              </div>

              <div className="showcase-card">
                <p className="showcase-question">"Show me all the logs associated with a failed trace"</p>
                <div className="showcase-viz">
                  <MockLogTable />
                </div>
              </div>

              <div className="showcase-card">
                <p className="showcase-question">"Where was the bottleneck across all spans in this action?"</p>
                <div className="showcase-viz">
                  <MockGantt />
                </div>
              </div>

            </div>
            <p style={{ textAlign: 'center', marginTop: '2rem' }}>
              <Link to="/docs/cookbook">See the Cookbook for more examples →</Link>
            </p>
          </div>
        </section>

        {/* ---------------------------------------------------------------- */}
        {/* Demo query                                                        */}
        {/* ---------------------------------------------------------------- */}
        <section style={{ padding: '3rem 0', borderBottom: '1px solid var(--ifm-color-emphasis-200)' }}>
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '0.5rem' }}>
              One query, multiple signals
            </h2>
            <p style={{ textAlign: 'center', color: 'var(--ifm-color-emphasis-700)', marginBottom: '2rem' }}>
              Find every joint limit breach that happened while the battery was critically low —
              scoped to your robot, auto-bounded to the last deploy. ROSQL converts units,
              understands lifecycle anchors, and correlates topic streams in a single pass. No JOINs.
            </p>
            <div style={{ maxWidth: 720, margin: '0 auto' }}>
              <CodeBlock language="sql" title="Cross-signal correlation with robot-native scoping">
                {DURING_QUERY}
              </CodeBlock>
            </div>
          </div>
        </section>

        {/* ---------------------------------------------------------------- */}
        {/* TIMESERIES + FACET demo                                           */}
        {/* ---------------------------------------------------------------- */}
        <section style={{ padding: '3rem 0', borderBottom: '1px solid var(--ifm-color-emphasis-200)' }}>
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '0.5rem' }}>
              Fleet-wide time-series, one line
            </h2>
            <p style={{ textAlign: 'center', color: 'var(--ifm-color-emphasis-700)', marginBottom: '2rem' }}>
              Group by any field with <code>FACET</code>. Bucket into intervals with <code>TIMESERIES</code>.
              Results include a <code>format_hint</code> so your UI renders a stacked chart automatically.
            </p>
            <div style={{ maxWidth: 720, margin: '0 auto' }}>
              <CodeBlock language="sql" title="Multi-robot CPU over time">
                {TIMESERIES_QUERY}
              </CodeBlock>
            </div>
          </div>
        </section>

        {/* ---------------------------------------------------------------- */}
        {/* Unit-aware filtering                                              */}
        {/* ---------------------------------------------------------------- */}
        <section style={{ padding: '3rem 0', borderBottom: '1px solid var(--ifm-color-emphasis-200)' }}>
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '0.5rem' }}>
              Sensor units — inline, auto-converted
            </h2>
            <p style={{ textAlign: 'center', color: 'var(--ifm-color-emphasis-700)', marginBottom: '2rem' }}>
              Write physical quantities in whatever unit makes sense. ROSQL normalizes to SI before hitting the database — no manual conversion, no unit mismatch bugs.
            </p>
            <div style={{ maxWidth: 900, margin: '0 auto', display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '1rem' }}>
              {[
                { expr: "fields['voltage'] < 11.5 V",   note: 'Catch battery sag',                    si: '→ 11.5 V' },
                { expr: "fields['distance'] < 24 in",   note: 'Proximity alert',                       si: '→ 0.61 m' },
                { expr: "fields['joint_angle'] > 30 deg", note: 'Joint limit warning',                 si: '→ 0.524 rad' },
                { expr: "fields['speed'] > 3.5 mph",    note: 'Flag velocity outliers',                si: '→ 1.56 m/s' },
              ].map(({ expr, note, si }) => (
                <div key={expr} style={{ background: 'var(--ifm-color-emphasis-100)', borderRadius: 8, padding: '1rem', borderLeft: '3px solid var(--ifm-color-primary)' }}>
                  <code style={{ fontSize: '0.8rem', display: 'block', marginBottom: '0.3rem', wordBreak: 'break-all' }}>{expr}</code>
                  <span style={{ fontSize: '0.72rem', color: 'var(--ifm-color-primary-lighter)', display: 'block', marginBottom: '0.25rem', fontFamily: 'var(--ifm-font-family-monospace)' }}>{si}</span>
                  <span style={{ fontSize: '0.8rem', color: 'var(--ifm-color-emphasis-600)' }}>{note}</span>
                </div>
              ))}
            </div>
          </div>
        </section>

        {/* ---------------------------------------------------------------- */}
        {/* Architecture                                                      */}
        {/* ---------------------------------------------------------------- */}
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

        {/* ---------------------------------------------------------------- */}
        {/* Driver support                                                    */}
        {/* ---------------------------------------------------------------- */}
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
                  {DRIVERS.map(({ name, flag, status, version, issue }) => (
                    <tr key={flag}>
                      <td>{name}</td>
                      <td><code>{flag}</code></td>
                      <td><StatusBadge status={status} version={version} issue={issue} /></td>
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

        {/* ---------------------------------------------------------------- */}
        {/* Quick start                                                       */}
        {/* ---------------------------------------------------------------- */}
        <section style={{ padding: '3rem 0', borderBottom: '1px solid var(--ifm-color-emphasis-200)' }}>
          <div className="container">
            <h2 style={{ textAlign: 'center', marginBottom: '0.5rem' }}>Quick start</h2>
            <p style={{ textAlign: 'center', color: 'var(--ifm-color-emphasis-600)', marginBottom: '2rem', fontSize: '0.9rem' }}>
              Linux · macOS (Intel &amp; Apple Silicon) · Windows via cargo
            </p>
            <div style={{ maxWidth: 720, margin: '0 auto' }}>
              <Tabs>
                <TabItem value="curl" label="curl | sh" default>
                  <CodeBlock language="bash">
                    {`curl -fsSL https://rosql.org/install.sh | sh`}
                  </CodeBlock>
                  <p style={{ color: 'var(--ifm-color-emphasis-600)', fontSize: '0.85rem', marginTop: '0.5rem' }}>
                    Linux x86_64 / arm64 · macOS Intel &amp; Apple Silicon. Installs to <code>~/.local/bin/</code>.
                  </p>
                </TabItem>
                <TabItem value="brew" label="Homebrew (macOS)">
                  <CodeBlock language="bash">
                    {`brew install robotopsinc/tap/rosql`}
                  </CodeBlock>
                  <p style={{ color: 'var(--ifm-color-emphasis-600)', fontSize: '0.85rem', marginTop: '0.5rem' }}>
                    Intel &amp; Apple Silicon. Supports <code>brew upgrade rosql</code>.
                  </p>
                </TabItem>
                <TabItem value="cargo" label="cargo install">
                  <CodeBlock language="bash">
                    {`cargo install rosql --features server,duckdb`}
                  </CodeBlock>
                  <p style={{ color: 'var(--ifm-color-emphasis-600)', fontSize: '0.85rem', marginTop: '0.5rem' }}>
                    All platforms including Windows. Requires Rust stable — <a href="https://rustup.rs">rustup.rs</a>.
                  </p>
                </TabItem>
              </Tabs>
              <div style={{ marginTop: '1.5rem' }}>
              <CodeBlock language="bash" title="Run your first query">
                {`rosql query "FROM traces WHERE status = 'ERROR' LIMIT 5" \\
  --backend parquet \\
  --url s3://robotops-production-rosql-demo/data`}
              </CodeBlock>
              </div>
            </div>
            <div style={{ display: 'flex', justifyContent: 'center', gap: 12, marginTop: '1.5rem', flexWrap: 'wrap' }}>
              <Link className="button button--primary" to="/docs/quickstart">Full quickstart →</Link>
              <Link className="button button--secondary" to="/docs/wasm">WASM / Browser →</Link>
            </div>
          </div>
        </section>

        {/* ---------------------------------------------------------------- */}
        {/* Robot Ops CTA                                                     */}
        {/* ---------------------------------------------------------------- */}
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
