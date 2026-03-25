import type { ReactNode } from 'react';
import Layout from '@theme/Layout';
import React, { useState } from 'react';

type FaqItem = {
  q: string;
  a: ReactNode;
};

const FAQS: FaqItem[] = [
  {
    q: 'How do you pronounce ROSQL?',
    a: '"RAW-skul" — Robot Ops Structured Query Language.',
  },
  {
    q: 'What is ROSQL?',
    a: (
      <>
        ROSQL is an open source SQL-like query language purpose-built for ROS2 telemetry data stored via
        OpenTelemetry. It lets robotics engineers query traces, logs, and metrics using familiar syntax with
        first-class support for ROS2 concepts — nodes, actions, topics, and message causality. It compiles
        to SQL and executes against your existing database.
      </>
    ),
  },
  {
    q: 'Does ROSQL replace SQL?',
    a: (
      <>
        No — ROSQL <em>compiles to SQL</em> and other database languages. When you write a ROSQL query, the library parses it and generates
        the equivalent query for your target database. ROSQL adds robotics semantics on top of SQL: duration
        units, <code>DURING()</code> cross-signal correlation, <code>MESSAGE JOURNEY</code> causality traversal,
        <code>HEALTH()</code>, <code>ANOMALY()</code>, and more. You can use <code>rosql compile</code> to
        see exactly what SQL a ROSQL query generates.
      </>
    ),
  },
  {
    q: 'How do I get ROS2 data into a ROSQL-compatible database?',
    a: (
      <>
        You need a ROS2-to-OTel bridge that exports traces, logs, and metrics in the{' '}
        <a href="/docs/schema-reference">expected schema</a>. Options:
        <ul>
          <li>
            <strong>Robot Ops Agent</strong> — managed ingestion with full OTel support.{' '}
            <a href="https://robotops.com">robotops.com</a>
          </li>
          <li>
            <strong>OTel Collector (community)</strong> — set up an OTel Collector with a ROS2 receiver
            and configure it to export to your database. You'll need to implement the span attribute conventions
            from the <a href="/docs/schema-reference">Schema Reference</a>.
          </li>
        </ul>
        The <code>MESSAGE JOURNEY</code> feature additionally requires that your instrumentation sets
        the publish span's <code>SpanId</code> as the <code>ParentSpanId</code> of the subscribe span.
      </>
    ),
  },
  {
    q: 'What databases does ROSQL support?',
    a: (
      <>
        PostgreSQL (including with TimescaleDB) and MySQL/MariaDB are supported in v0.1. DuckDB,
        AWS Athena, Google BigQuery, and InfluxDB are in development. See the <a href="/docs/drivers">Driver Support</a> page.
      </>
    ),
  },
  {
    q: 'Is ROSQL free?',
    a: (
      <>
        Yes. ROSQL is licensed under Apache 2.0. The full language, parser, AST, compiler,
        and all drivers are open source. See the{' '}
        <a href="https://github.com/RobotOpsInc/rosql/blob/main/LICENSE">LICENSE</a> file.
      </>
    ),
  },
  {
    q: 'Who created ROSQL?',
    a: (
      <>
        ROSQL was created by <a href="https://robotops.com">Robot Ops, Inc.</a> and is used to power
        the Robot Ops observability platform. It is open source and community contributions are welcome.
        ROSQL is a trademark of Robot Ops, Inc.
      </>
    ),
  },
  {
    q: 'Can I use ROSQL in the browser?',
    a: (
      <>
        Yes. The <a href="/docs/wasm"><code>@robotops/rosql</code></a> npm package provides a WebAssembly
        build that runs in any modern browser. It exposes <code>parse()</code>, <code>validate()</code>,
        and <code>get_completions()</code> — useful for building editor integrations and live query validation.
        It does <em>not</em> execute queries against a database (WASM has no direct DB access); for that,
        use the CLI or library.
      </>
    ),
  },
  {
    q: 'What ROS2 versions are supported?',
    a: (
      <>
        Any ROS2 version that can be instrumented with OpenTelemetry. ROSQL doesn't directly interface with
        ROS2 — it queries the OTel telemetry that your ROS2 nodes emit. The schema conventions are defined
        in the <a href="/docs/schema-reference">Schema Reference</a> and are version-agnostic.
      </>
    ),
  },
  {
    q: 'How does ROSQL handle duration units?',
    a: (
      <>
        ROSQL automatically converts duration expressions to nanoseconds (the storage unit in <code>otel_traces</code>):
        <code>500 ms → 500000000</code>, <code>2 s → 2000000000</code>, <code>1 min → 60000000000</code>.
        You write human-readable durations; ROSQL handles the conversion.
      </>
    ),
  },
  {
    q: 'Can I contribute to ROSQL?',
    a: (
      <>
        Yes! See the <a href="/contributing">Contributing</a> page or jump straight to the{' '}
        <a href="https://github.com/RobotOpsInc/rosql/blob/main/CONTRIBUTING.md">CONTRIBUTING.md</a>.
        File bugs and feature requests in the{' '}
        <a href="https://github.com/RobotOpsInc/rosql/issues">issue tracker</a>.
      </>
    ),
  },
  {
    q: 'What is the relationship between ROSQL and the Robot Ops platform?',
    a: (
      <>
        ROSQL is the query layer that Robot Ops, Inc. built and uses internally to power the{' '}
        <a href="https://robotops.com">Robot Ops observability platform</a>. The open source crate includes
        the full language, compiler, and open source drivers (PostgreSQL, MySQL). The Robot Ops platform adds
        managed ingestion, fleet-scale storage (ClickHouse), lifecycle anchors, fleet-wide anomaly detection,
        and dashboards. Think of ROSQL as the query language, and Robot Ops as one platform that uses it.
      </>
    ),
  },
];

function FaqAccordion({ items }: { items: FaqItem[] }) {
  const [openIndex, setOpenIndex] = useState<number | null>(null);

  return (
    <div>
      {items.map(({ q, a }, i) => (
        <div
          key={i}
          style={{
            borderBottom: '1px solid var(--ifm-color-emphasis-200)',
          }}
        >
          <button
            onClick={() => setOpenIndex(openIndex === i ? null : i)}
            style={{
              width: '100%',
              background: 'none',
              border: 'none',
              textAlign: 'left',
              padding: '1.1rem 0',
              fontSize: '1rem',
              fontWeight: 600,
              cursor: 'pointer',
              color: 'var(--ifm-font-color-base)',
              display: 'flex',
              justifyContent: 'space-between',
              alignItems: 'center',
              gap: 12,
            }}
            aria-expanded={openIndex === i}
          >
            <span>{q}</span>
            <span style={{ fontSize: '1.25rem', lineHeight: 1, flexShrink: 0, color: 'var(--ifm-color-emphasis-600)' }}>
              {openIndex === i ? '−' : '+'}
            </span>
          </button>
          {openIndex === i && (
            <div style={{ paddingBottom: '1.25rem', color: 'var(--ifm-color-emphasis-800)', lineHeight: 1.7 }}>
              {a}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

export default function FAQ(): ReactNode {
  return (
    <Layout
      title="FAQ"
      description="Frequently asked questions about ROSQL — pronunciation, database support, licensing, Robot Ops, and more"
    >
      <div className="container" style={{ padding: '2.5rem 0 4rem' }}>
        <div style={{ maxWidth: 760, margin: '0 auto' }}>
          <h1>Frequently asked questions</h1>
          <p style={{ fontSize: '1.05rem', color: 'var(--ifm-color-emphasis-700)', marginBottom: '2.5rem' }}>
            Can't find your answer? Email{' '}
            <a href="mailto:devs@robotops.com">devs@robotops.com</a> or{' '}
            <a href="https://github.com/RobotOpsInc/rosql/issues">open an issue</a>.
          </p>
          <FaqAccordion items={FAQS} />
        </div>
      </div>
    </Layout>
  );
}
