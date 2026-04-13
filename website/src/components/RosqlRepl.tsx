import React, { useState, useEffect, useRef, useCallback } from 'react';
import BrowserOnly from '@docusaurus/BrowserOnly';
import type { VisualizationConfig } from './visualizations/types';
import { ResultRenderer } from './visualizations/ResultRenderer';

const EXAMPLE_QUERIES = [
  {
    label: 'Trace a failed mission',
    query: "TRACE 'trace-amr02-m3'",
  },
  {
    label: 'What went wrong?',
    query: "TRACE 'trace-amr02-m3'\nENRICH WITH logs LIMIT 5",
  },
  {
    label: 'Fleet CPU over time',
    query: "SELECT cpu_usage FROM metrics\nTIMESERIES 2 min FACET robot_id\nSINCE 45 min ago",
  },
  {
    label: 'Message flow: /scan',
    query: "MESSAGE FLOW FROM TOPIC '/scan'\nFOR ROBOT 'robot-amr-02'",
  },
  {
    label: 'Slowest spans',
    query: "SHOW SPAN SUMMARY\nFOR ROBOT 'robot-amr-02'\nSINCE 90 min ago",
  },
  {
    label: 'Path deviation',
    query: "PATH DEVIATION\nFOR TRACE 'trace-amr02-m3'",
  },
  {
    label: 'Which robot regressed?',
    query: "ANOMALY(duration)\nCOMPARED TO last week\nFACET robot_id",
  },
  {
    label: 'Battery below 11.5V',
    query: "FROM topics\nWHERE topic_name = '/battery_state'\n  AND fields['voltage'] < 11.5 V\nFOR ROBOT 'robot-amr-02'\nSINCE 2 h ago",
  },
  {
    label: 'ROS2 node topology',
    query: "SHOW NODE GRAPH\nFOR ROBOT 'robot-amr-02'",
  },
];

const FIXTURE_FILES = [
  '/fixtures/01_schema.sql',
  '/fixtures/02_traces.sql',
  '/fixtures/03_logs.sql',
  '/fixtures/04_metrics.sql',
  '/fixtures/05_topic_messages.sql',
  '/fixtures/06_mcap_metadata.sql',
  '/fixtures/07_events.sql',
  '/fixtures/08_baseline.sql',
];

type OutputState =
  | { kind: 'idle' }
  | { kind: 'loading'; message: string }
  | { kind: 'success'; rows: Record<string, unknown>[]; rawJson: string; sql?: string; formatHint: string; visualization?: VisualizationConfig }
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
  const [visualMode, setVisualMode] = useState(true);
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
    setVisualMode(true);
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
    setVisualMode(true);

    // Compile ROSQL → SQL
    const compileResult = normalize(rosqlRef.current.compile(query)) as {
      ok: boolean;
      sql?: string;
      error?: { message: string };
      format_hint?: string;
      visualization?: VisualizationConfig;
    };
    if (!compileResult.ok) {
      setOutput({ kind: 'error', text: `Compile error: ${compileResult.error?.message ?? 'unknown'}` });
      return;
    }
    const sql = compileResult.sql!;
    const formatHint = compileResult.format_hint ?? 'Table';
    const visualization = compileResult.visualization ?? undefined;

    // Ensure DuckDB is ready
    const ready = await initDb();
    if (!ready || !dbRef.current) return;

    setOutput({ kind: 'loading', message: 'Running query…' });
    try {
      const rawRows = await dbRef.current.query(sql);
      const rows = rawRows.map((r) =>
        Object.fromEntries(
          Object.entries(r as Record<string, unknown>).map(([k, v]) => [k, typeof v === 'bigint' ? Number(v) : v])
        )
      ) as Record<string, unknown>[];
      const rawJson = JSON.stringify(rows, null, 2);
      setOutput({ kind: 'success', rows, rawJson, sql, formatHint, visualization });
    } catch (err) {
      setOutput({ kind: 'error', text: String(err), sql });
    }
  }, [query, initDb]);

  const isRunning = output.kind === 'loading';
  const compiledSql = output.kind === 'success' || output.kind === 'error' ? output.sql : undefined;
  const successOutput = output.kind === 'success' ? output : null;

  // Whether the Visual toggle is relevant for this result
  const hasVisualMode = successOutput !== null && successOutput.formatHint !== 'Table';
  const showVisualOutput = successOutput !== null && visualMode;

  const idleText = rosqlReady ? '← Click Run to execute against sample data' : 'Loading WASM parser…';

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
      </div>
      <div className="rosql-repl-panes">
        <div className="rosql-repl-editor" ref={editorRef} />
        <div className="rosql-repl-output" style={{ background: '#0F0F0F' }}>
          {/* Output pane header: Visual / Raw JSON toggle */}
          {successOutput !== null && (
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '6px 10px', borderBottom: '1px solid #1F2937' }}>
              <div style={{ display: 'flex', gap: 6 }}>
                <button
                  onClick={() => setVisualMode(true)}
                  style={{
                    background: visualMode ? '#1F2937' : 'transparent',
                    border: '1px solid ' + (visualMode ? '#374151' : 'transparent'),
                    borderRadius: 4,
                    color: visualMode ? '#D1D5DB' : '#6B7280',
                    fontSize: 11,
                    padding: '2px 8px',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                  }}
                >
                  Visual
                </button>
                <button
                  onClick={() => setVisualMode(false)}
                  style={{
                    background: !visualMode ? '#1F2937' : 'transparent',
                    border: '1px solid ' + (!visualMode ? '#374151' : 'transparent'),
                    borderRadius: 4,
                    color: !visualMode ? '#D1D5DB' : '#6B7280',
                    fontSize: 11,
                    padding: '2px 8px',
                    cursor: 'pointer',
                    fontFamily: 'inherit',
                  }}
                >
                  Raw JSON
                </button>
              </div>
              <span style={{ fontSize: 10, color: '#4B5563' }}>
                {successOutput.rows.length} row{successOutput.rows.length !== 1 ? 's' : ''} · {successOutput.formatHint}
              </span>
            </div>
          )}

          {/* Main output area */}
          <div style={{ padding: output.kind !== 'idle' && output.kind !== 'loading' ? '10px' : 0 }}>
            {output.kind === 'idle' && (
              <pre style={{ color: '#9CA3AF', background: 'transparent', margin: 0, padding: '12px' }}>{idleText}</pre>
            )}
            {output.kind === 'loading' && (
              <pre style={{ color: '#9CA3AF', background: 'transparent', margin: 0, padding: '12px' }}>{output.message}</pre>
            )}
            {output.kind === 'error' && (
              <pre style={{ color: '#FCA5A5', background: 'transparent', margin: 0, whiteSpace: 'pre-wrap', fontSize: 12 }}>{output.text}</pre>
            )}
            {output.kind === 'success' && showVisualOutput && (
              <ResultRenderer
                rows={output.rows}
                formatHint={output.formatHint}
                visualization={output.visualization}
              />
            )}
            {output.kind === 'success' && !showVisualOutput && (
              <pre style={{ color: '#6EE7B7', background: 'transparent', margin: 0, fontSize: 11, whiteSpace: 'pre-wrap', maxHeight: 280, overflowY: 'auto' }}>
                {output.rawJson}
              </pre>
            )}
          </div>

          {/* View compiled SQL collapsible */}
          {compiledSql && (
            <div style={{ marginTop: 4, borderTop: '1px solid #1F2937', paddingTop: 6, padding: '6px 10px' }}>
              <button
                onClick={() => setShowSql((v) => !v)}
                style={{ background: 'none', border: 'none', color: '#6B7280', fontSize: 11, cursor: 'pointer', padding: 0 }}
              >
                {showSql ? '▾ Hide SQL' : '▸ View compiled SQL'}
              </button>
              {showSql && (
                <pre style={{ marginTop: 6, color: '#9CA3AF', fontSize: 11, whiteSpace: 'pre-wrap', background: 'transparent' }}>
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
