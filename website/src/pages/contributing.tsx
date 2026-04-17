import type { ReactNode } from 'react';
import Layout from '@theme/Layout';
import CodeBlock from '@theme/CodeBlock';

export default function Contributing(): ReactNode {
  return (
    <Layout
      title="Contributing"
      description="Contribute to ROSQL — how to get started, build variants, submitting changes, and the release process"
    >
      <div className="container" style={{ padding: '2.5rem 0 4rem' }}>
        <div style={{ maxWidth: 760, margin: '0 auto' }}>
          <h1>Contributing to ROSQL</h1>
          <p style={{ fontSize: '1.1rem', color: 'var(--ifm-color-emphasis-700)', marginBottom: '2rem' }}>
            ROSQL is in early development and contributions are very welcome.
            Whether you're fixing a bug, adding a feature, improving docs, or just kicking the tires — thank you.
          </p>

          <h2>Prerequisites</h2>
          <ul>
            <li><strong>Rust</strong> (stable, 1.80+) — <a href="https://rustup.rs">rustup.rs</a></li>
            <li><strong>protoc</strong> — required for proto code generation (<code>brew install protobuf</code> on macOS, <code>apt-get install protobuf-compiler</code> on Debian/Ubuntu)</li>
            <li><strong>buf</strong> (optional) — for proto linting: <a href="https://buf.build/docs/installation">buf.build</a></li>
            <li><strong>just</strong> (optional) — command runner: <a href="https://just.systems">just.systems</a></li>
          </ul>

          <h2>Getting started</h2>
          <CodeBlock language="bash">
{`git clone https://github.com/RobotOpsInc/rosql
cd rosql
just build       # or: cargo build
just test        # or: cargo test`}
          </CodeBlock>

          <h2>Build variants</h2>
          <CodeBlock language="bash">
{`# Default: parser + drivers (no networking, no WASM)
cargo build

# WASM package (for frontend editors)
cargo build --target wasm32-unknown-unknown --features wasm

# gRPC server + CLI binary
cargo build --features server --bin rosql`}
          </CodeBlock>

          <h2>Running checks</h2>
          <CodeBlock language="bash">
{`just check       # runs build + test + clippy + fmt + buf-lint`}
          </CodeBlock>
          <p>Or individually:</p>
          <CodeBlock language="bash">
{`cargo test
cargo clippy -- -D warnings
cargo fmt --check
buf lint proto/`}
          </CodeBlock>

          <h2>Submitting changes</h2>
          <ol>
            <li>Fork the repo and create a feature branch from <code>development</code></li>
            <li>Make your changes</li>
            <li>Run <code>just check</code> and ensure everything passes</li>
            <li>Increment the version using <code>just bump-version [major|minor|patch]</code></li>
            <li>Open a pull request against <code>development</code></li>
          </ol>

          <h2>Proto development</h2>
          <p>Proto files live in <code>proto/rosql/v1/</code>. When you modify a <code>.proto</code> file:</p>
          <ol>
            <li><code>cargo build</code> — regenerates Rust types via <code>prost-build</code></li>
            <li><code>buf lint proto/</code> — validate proto style compliance</li>
            <li><code>cargo test</code> — ensure generated types compile and tests pass</li>
          </ol>

          <h2>Reporting issues</h2>
          <p>
            File bugs and feature requests in the{' '}
            <a href="https://github.com/RobotOpsInc/rosql/issues">issue tracker</a>.
            For questions, email{' '}
            <a href="mailto:devs@robotops.com">devs@robotops.com</a>.
          </p>

          <div style={{ marginTop: '2rem', padding: '1.5rem', background: 'var(--ifm-color-emphasis-100)', borderRadius: 8 }}>
            <strong>Full CONTRIBUTING.md</strong>:{' '}
            <a href="https://github.com/RobotOpsInc/rosql/blob/main/CONTRIBUTING.md">
              github.com/RobotOpsInc/rosql/blob/main/CONTRIBUTING.md
            </a>
          </div>

          <p style={{ marginTop: '1.5rem', color: 'var(--ifm-color-emphasis-600)', fontSize: '0.9rem' }}>
            By contributing, you agree that your contributions will be licensed under the Apache 2.0 license.
          </p>
        </div>
      </div>
    </Layout>
  );
}
