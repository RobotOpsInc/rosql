import type { ReactNode } from 'react';
import Layout from '@theme/Layout';
import { RosqlRepl } from '@site/src/components/RosqlRepl';

export default function Playground(): ReactNode {
  return (
    <Layout
      title="Try ROSQL"
      description="Interactive ROSQL playground — parse and validate queries in the browser using the WASM package"
    >
      <div className="container" style={{ padding: '2.5rem 0 4rem' }}>
        <div style={{ maxWidth: 960, margin: '0 auto' }}>
          <h1>Try ROSQL</h1>
          <p style={{ fontSize: '1rem', color: 'var(--ifm-color-emphasis-700)', marginBottom: '1.5rem' }}>
            Parse and validate ROSQL queries in the browser using the{' '}
            <a href="/docs/wasm"><code>@robotops/rosql</code></a> WASM package.
            Select an example or write your own query, then click <strong>Run</strong> to execute it against sample data.
          </p>
          <p style={{ fontSize: '0.875rem', color: 'var(--ifm-color-emphasis-600)', marginBottom: '2rem', fontStyle: 'italic' }}>
            Note: this playground parses queries only — it does not connect to a database. To execute queries
            against real data, use the <a href="/docs/cli">CLI</a> or <a href="/docs/quickstart">library</a>.
          </p>
          <RosqlRepl />
          <div style={{ marginTop: '3rem', padding: '1.5rem', background: 'var(--ifm-color-emphasis-100)', borderRadius: 8 }}>
            <h3 style={{ marginBottom: '0.75rem' }}>Run queries against your data</h3>
            <p style={{ color: 'var(--ifm-color-emphasis-700)', marginBottom: '1rem' }}>
              The WASM playground shows the parsed AST. To execute queries and get real results:
            </p>
            <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
              <a className="button button--primary" href="/docs/quickstart">Install the CLI →</a>
              <a className="button button--secondary" href="/docs/wasm">Embed in your app →</a>
            </div>
          </div>
        </div>
      </div>
    </Layout>
  );
}
