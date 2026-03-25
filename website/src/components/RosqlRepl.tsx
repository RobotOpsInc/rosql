import React, { useState, useEffect, useRef, useCallback } from 'react';
import BrowserOnly from '@docusaurus/BrowserOnly';

const EXAMPLE_QUERIES = [
  {
    label: 'Find error spans',
    query: "FROM traces WHERE status = 'ERROR' SINCE 1 hour ago",
  },
  {
    label: 'Cross-signal correlation (DURING)',
    query: `SELECT trace_id, span_name_col, service_name, duration, status_code
FROM traces
WHERE status = 'ERROR' AND action_name = '/navigate_to_pose'
DURING(
  FROM topics WHERE topic_name = '/battery_state'
  AND fields['percentage'] < 15
)
SINCE 6 hours ago`,
  },
  {
    label: 'Message causality chain',
    query: "MESSAGE JOURNEY FOR TRACE 'a3f1c9d2e8b04f7a'",
  },
  {
    label: 'Robot health assessment',
    query: "HEALTH() FOR ROBOT 'robot_sim_001'",
  },
  {
    label: 'Anomaly detection',
    query: 'ANOMALY(duration)',
  },
  {
    label: 'Pipeline syntax',
    query: `FROM traces
| WHERE duration > 500 ms
| WHERE status = 'ERROR'
| FACET robot_id
| COMPARE TO last week`,
  },
];

type OutputState =
  | { kind: 'idle' }
  | { kind: 'loading' }
  | { kind: 'success'; text: string }
  | { kind: 'error'; text: string };

function RosqlReplInner({ compact = false }: { compact?: boolean }) {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [query, setQuery] = useState(EXAMPLE_QUERIES[0].query);
  const [output, setOutput] = useState<OutputState>({ kind: 'idle' });
  const [wasmReady, setWasmReady] = useState(false);
  const rosqlRef = useRef<{
    parse: (q: string) => string;
    validate: (q: string) => string;
  } | null>(null);
  const editorRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<{ destroy: () => void; state: { doc: { toString: () => string } } } | null>(null);

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
              if (update.docChanged) {
                setQuery(update.state.doc.toString());
              }
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
    return () => {
      destroyed = true;
      viewRef.current?.destroy();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Load WASM
  useEffect(() => {
    import('@robotops/rosql').then(async (mod) => {
      await mod.default('/rosql_bg.wasm');
      rosqlRef.current = mod as unknown as typeof rosqlRef.current;
      setWasmReady(true);
    }).catch(() => {
      // WASM load failed silently — buttons stay disabled
    });
  }, []);

  const handleExampleChange = useCallback((e: React.ChangeEvent<HTMLSelectElement>) => {
    const idx = Number(e.target.value);
    setSelectedIndex(idx);
    const newQuery = EXAMPLE_QUERIES[idx].query;
    setQuery(newQuery);
    setOutput({ kind: 'idle' });
    // Update CodeMirror content
    if (viewRef.current) {
      const { EditorState } = require('@codemirror/state');
      // We can't easily update the view without a full reinit, so the textarea approach
      // is simpler. We'll update via a state transaction.
      try {
        const view = viewRef.current as unknown as { dispatch: (tr: unknown) => void; state: { doc: { length: number } } };
        view.dispatch({
          changes: { from: 0, to: view.state.doc.length, insert: newQuery },
        });
      } catch {
        // ignore
      }
    }
  }, []);

  const run = useCallback((mode: 'parse' | 'validate') => {
    if (!rosqlRef.current) return;
    setOutput({ kind: 'loading' });
    try {
      const raw = mode === 'parse'
        ? rosqlRef.current.parse(query)
        : rosqlRef.current.validate(query);
      const parsed = JSON.parse(raw);
      setOutput({ kind: 'success', text: JSON.stringify(parsed, null, 2) });
    } catch (err) {
      setOutput({ kind: 'error', text: String(err) });
    }
  }, [query]);

  const outputText = (() => {
    switch (output.kind) {
      case 'idle': return wasmReady ? '← Click Parse or Validate to run' : 'Loading WASM parser…';
      case 'loading': return 'Running…';
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
        <button onClick={() => run('parse')} disabled={!wasmReady || output.kind === 'loading'}>
          Parse
        </button>
        <button onClick={() => run('validate')} disabled={!wasmReady || output.kind === 'loading'}>
          Validate
        </button>
      </div>
      <div className="rosql-repl-panes">
        <div className="rosql-repl-editor" ref={editorRef} />
        <div className="rosql-repl-output">
          <pre className={output.kind === 'error' ? 'error' : output.kind === 'success' ? 'success' : ''}>
            {outputText}
          </pre>
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
