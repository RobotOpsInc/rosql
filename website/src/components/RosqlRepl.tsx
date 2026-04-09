import React, { useState, useEffect, useRef, useCallback } from 'react';
import BrowserOnly from '@docusaurus/BrowserOnly';

const EXAMPLE_QUERIES = [
  {
    label: 'Find error traces',
    query: "FROM traces WHERE status = 'ERROR'\n-- SINCE 1 hour ago",
  },
  {
    label: 'Message causality chain',
    query: "MESSAGE JOURNEY FOR TRACE 'trace-002'",
  },
  {
    label: 'Error rate by robot',
    query: "SELECT COUNT(*) FROM traces WHERE status = 'ERROR' FACET robot_id\n-- SINCE 30 minutes ago",
  },
  {
    label: 'Action success rate',
    query: "SELECT ACTION_SUCCESS_RATE('/navigate_to_pose') FROM traces\n-- SINCE 1 hour ago",
  },
  {
    label: 'Topic messages',
    query: "FROM topics WHERE topic_name = '/battery_state'\n-- SINCE 1 hour ago",
  },
  {
    label: 'Pipeline syntax',
    query: `FROM traces
| WHERE status = 'ERROR'
| FACET service_name
-- | SINCE 1 hour ago`,
  },
];

const FIXTURE_FILES = [
  '/fixtures/01_schema.sql',
  '/fixtures/02_traces.sql',
  '/fixtures/03_logs.sql',
  '/fixtures/04_metrics.sql',
  '/fixtures/05_topic_messages.sql',
  '/fixtures/06_mcap_metadata.sql',
];

type OutputState =
  | { kind: 'idle' }
  | { kind: 'loading'; message: string }
  | { kind: 'success'; text: string; sql?: string }
  | { kind: 'error'; text: string; sql?: string };

/** Recursively convert JS Maps (from serde_wasm_bindgen) to plain objects */
function normalize(val: unknown): unknown {
  if (val instanceof Map) return Object.fromEntries([...val].map(([k, v]) => [k, normalize(v)]));
  if (Array.isArray(val)) return val.map(normalize);
  return val;
}

function RosqlReplInner({ compact = false }: { compact?: boolean }) {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [query, setQuery] = useState(EXAMPLE_QUERIES[0].query);
  const [output, setOutput] = useState<OutputState>({ kind: 'idle' });
  const [showSql, setShowSql] = useState(false);
  const [rosqlReady, setRosqlReady] = useState(false);
  const [dbReady, setDbReady] = useState(false);
  const [dbLoading, setDbLoading] = useState(false);

  const rosqlRef = useRef<{
    parse: (q: string) => unknown;
    validate: (q: string) => unknown;
    compile: (q: string) => unknown;
  } | null>(null);
  const dbRef = useRef<{ query: (sql: string) => Promise<unknown[]> } | null>(null);
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<{ destroy: () => void; state: { doc: { length: number } } } | null>(null);

  // Initialize CodeMirror
  useEffect(() => {
    let destroyed = false;
    async function init() {
      const [{ EditorView, keymap, lineNumbers, highlightActiveLine }, { EditorState }, { sql }, { defaultKeymap }, { oneDark }] =
        await Promise.all([
          import('@codemirror/view'),
          import('@codemirror/state'),
          import('@codemirror/lang-sql'),
          import('@codemirror/commands'),
          import('@codemirror/theme-one-dark'),
        ]);
      if (destroyed || !editorRef.current) return;
      const view = new EditorView({
        state: EditorState.create({
          doc: query,
          extensions: [
            oneDark,
            lineNumbers(),
            highlightActiveLine(),
            sql(),
            keymap.of(defaultKeymap),
            EditorView.updateListener.of((update) => {
              if (update.docChanged) setQuery(update.state.doc.toString());
            }),
            EditorView.theme({
              '&': { height: compact ? '160px' : '240px', fontSize: '13px' },
              '.cm-scroller': { overflow: 'auto' },
            }),
          ],
        }),
        parent: editorRef.current,
      });
      viewRef.current = view as unknown as typeof viewRef.current;
    }
    init();
    return () => { destroyed = true; viewRef.current?.destroy(); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Load ROSQL WASM
  useEffect(() => {
    import('@robotops/rosql').then(async (mod) => {
      await mod.default('/rosql_bg.wasm');
      rosqlRef.current = mod as unknown as typeof rosqlRef.current;
      setRosqlReady(true);
    }).catch(() => {
      setOutput({ kind: 'error', text: 'Failed to load ROSQL WASM parser.' });
    });
  }, []);

  // Lazily load DuckDB + fixtures on first Run click
  const initDb = useCallback(async () => {
    if (dbRef.current) return true;
    setDbLoading(true);
    setOutput({ kind: 'loading', message: 'Loading DuckDB…' });
    try {
      const duckdb = await import('@duckdb/duckdb-wasm');
      const JSDELIVR_BUNDLES = duckdb.getJsDelivrBundles();
      const bundle = await duckdb.selectBundle(JSDELIVR_BUNDLES);
      const worker_url = URL.createObjectURL(
        new Blob([`importScripts("${bundle.mainWorker!}");`], { type: 'text/javascript' })
      );
      const worker = new Worker(worker_url);
      const logger = new duckdb.ConsoleLogger(duckdb.LogLevel.WARNING);
      const db = new duckdb.AsyncDuckDB(logger, worker);
      await db.instantiate(bundle.mainModule, bundle.pthreadWorker);
      URL.revokeObjectURL(worker_url);

      const conn = await db.connect();

      // Load fixture SQL files
      setOutput({ kind: 'loading', message: 'Loading fixture data…' });
      for (const file of FIXTURE_FILES) {
        const res = await fetch(file);
        const sql = await res.text();
        await conn.query(sql);
      }

      // Wrap connection with a simple query helper
      dbRef.current = {
        query: async (sql: string) => {
          const result = await conn.query(sql);
          return result.toArray().map((row) => row.toJSON());
        },
      };
      setDbReady(true);
      setDbLoading(false);
      return true;
    } catch (err) {
      setOutput({ kind: 'error', text: `Failed to load DuckDB: ${String(err)}` });
      setDbLoading(false);
      return false;
    }
  }, []);

  const handleExampleChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    const idx = Number(e.target.value);
    setSelectedIndex(idx);
    const newQuery = EXAMPLE_QUERIES[idx].query;
    setQuery(newQuery);
    setOutput({ kind: 'idle' });
    setShowSql(false);
    if (viewRef.current) {
      try {
        const view = viewRef.current as unknown as { dispatch: (tr: unknown) => void; state: { doc: { length: number } } };
        view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: newQuery } });
      } catch { /* ignore */ }
    }
  }, []);

  const handleRun = useCallback(async () => {
    if (!rosqlRef.current) return;
    setShowSql(false);

    // Compile ROSQL → SQL
    const compileResult = normalize(rosqlRef.current.compile(query)) as { ok: boolean; sql?: string; error?: { message: string } };
    if (!compileResult.ok) {
      setOutput({ kind: 'error', text: `Compile error: ${compileResult.error?.message ?? 'unknown'}` });
      return;
    }
    const sql = compileResult.sql!;

    // Ensure DuckDB is ready
    const ready = await initDb();
    if (!ready || !dbRef.current) return;

    setOutput({ kind: 'loading', message: 'Running query…' });
    try {
      const rows = await dbRef.current.query(sql);
      setOutput({ kind: 'success', text: JSON.stringify(rows, (_, v) => typeof v === 'bigint' ? Number(v) : v, 2), sql });
    } catch (err) {
      setOutput({ kind: 'error', text: String(err), sql });
    }
  }, [query, initDb]);

  const handleValidate = useCallback(() => {
    if (!rosqlRef.current) return;
    try {
      const result = normalize(rosqlRef.current.validate(query)) as { valid: boolean; errors: { message: string }[] };
      if (result.valid) {
        setOutput({ kind: 'success', text: '✓ Query is valid' });
      } else {
        const msgs = result.errors.map((e) => e.message).join('\n');
        setOutput({ kind: 'error', text: msgs });
      }
    } catch (err) {
      setOutput({ kind: 'error', text: String(err) });
    }
  }, [query]);

  const isRunning = output.kind === 'loading';
  const compiledSql = output.kind === 'success' || output.kind === 'error' ? output.sql : undefined;

  const outputText = (() => {
    switch (output.kind) {
      case 'idle': return rosqlReady ? '← Click Run to execute against sample data' : 'Loading WASM parser…';
      case 'loading': return output.message;
      case 'success': return output.text;
      case 'error': return output.text;
    }
  })();

  return (
    <div className="rosql-repl">
      <div className="rosql-repl-toolbar">
        <select value={selectedIndex} onChange={handleExampleChange} aria-label="Example query">
          {EXAMPLE_QUERIES.map((q, i) => (
            <option key={i} value={i}>{q.label}</option>
          ))}
        </select>
        <button
          onClick={handleRun}
          disabled={!rosqlReady || isRunning}
          style={{ background: 'var(--ifm-color-primary)', fontWeight: 600 }}
        >
          {isRunning ? '…' : 'Run'}
        </button>
        <button onClick={handleValidate} disabled={!rosqlReady || isRunning}>
          Validate
        </button>
      </div>
      <div className="rosql-repl-panes">
        <div className="rosql-repl-editor" ref={editorRef} />
        <div className="rosql-repl-output">
          <pre className={output.kind === 'error' ? 'error' : output.kind === 'success' ? 'success' : ''}>
            {outputText}
          </pre>
          {compiledSql && (
            <div style={{ marginTop: 8, borderTop: '1px solid #2A2A2A', paddingTop: 8 }}>
              <button
                onClick={() => setShowSql((v) => !v)}
                style={{ background: 'none', border: 'none', color: '#6B7280', fontSize: 11, cursor: 'pointer', padding: 0 }}
              >
                {showSql ? '▾ Hide SQL' : '▸ View compiled SQL'}
              </button>
              {showSql && (
                <pre style={{ marginTop: 6, color: '#9CA3AF', fontSize: 11, whiteSpace: 'pre-wrap' }}>
                  {compiledSql}
                </pre>
              )}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export function RosqlRepl({ compact = false }: { compact?: boolean }) {
  return (
    <BrowserOnly fallback={<div className="rosql-repl" style={{ height: '200px', display: 'flex', alignItems: 'center', justifyContent: 'center', color: '#94a3b8' }}>Loading editor…</div>}>
      {() => <RosqlReplInner compact={compact} />}
    </BrowserOnly>
  );
}
