import type { ReactNode } from 'react';
import Layout from '@theme/Layout';

export default function Benchmarks(): ReactNode {
  return (
    <Layout
      title="Benchmarks"
      description="ROSQL performance benchmarks — parser throughput, compilation time, end-to-end latency, WASM bundle size"
    >
      <div className="container" style={{ padding: '2.5rem 0 4rem' }}>
        <div style={{ maxWidth: 760, margin: '0 auto' }}>
          <h1>Benchmarks</h1>
          <p style={{ fontSize: '1.05rem', color: 'var(--ifm-color-emphasis-700)', marginBottom: '2rem' }}>
            Performance benchmarks for ROSQL are in progress. Results coming soon.
          </p>

          <div style={{ padding: '1.5rem', background: 'var(--ifm-color-emphasis-100)', borderRadius: 8, marginBottom: '2.5rem' }}>
            <strong>Tracking issue:</strong>{' '}
            <a href="https://github.com/RobotOpsInc/rosql/issues/35">#35 — Benchmarks</a>
          </div>

          <h2>Planned benchmark categories</h2>

          <table>
            <thead>
              <tr>
                <th>Category</th>
                <th>What it measures</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Parser throughput</td>
                <td>Queries parsed per second</td>
                <td>🔜 In progress</td>
              </tr>
              <tr>
                <td>SQL compilation time</td>
                <td>ns per query, by dialect and complexity</td>
                <td>🔜 In progress</td>
              </tr>
              <tr>
                <td>End-to-end latency</td>
                <td>parse → compile → execute (full round-trip)</td>
                <td>🔜 In progress</td>
              </tr>
              <tr>
                <td>WASM parse latency</td>
                <td>Browser-side parse time</td>
                <td>🔜 In progress</td>
              </tr>
              <tr>
                <td>WASM bundle size</td>
                <td>gzipped size of <code>@robotops/rosql</code></td>
                <td>🔜 In progress</td>
              </tr>
            </tbody>
          </table>

          <h2 style={{ marginTop: '2.5rem' }}>Methodology</h2>
          <p style={{ color: 'var(--ifm-color-emphasis-700)' }}>
            Benchmarks will be run using{' '}
            <a href="https://github.com/bheisler/criterion.rs">criterion.rs</a> for Rust benchmarks
            and a custom harness for WASM. All results will include:
          </p>
          <ul style={{ color: 'var(--ifm-color-emphasis-700)' }}>
            <li>Hardware configuration (CPU, memory)</li>
            <li>Query complexity categories (simple, compound, pipeline)</li>
            <li>Comparison across SQL backends where applicable</li>
            <li>Statistical confidence intervals</li>
          </ul>
          <p style={{ color: 'var(--ifm-color-emphasis-700)' }}>
            Follow <a href="https://github.com/RobotOpsInc/rosql/issues/35">#35</a> for updates.
          </p>
        </div>
      </div>
    </Layout>
  );
}
