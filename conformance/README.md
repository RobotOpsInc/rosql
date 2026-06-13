# ROSQL driver-parity conformance suite (ROB-297, AC-Q1)

`conformance/cases.json` is a **shared, machine-readable contract** that keeps the
OSS `rosql` crate (parser + `src/drivers/` conventions) and the **go_backend
ClickHouse planner** from drifting on:

- **Data-source → table mappings** (AC-P4): `tf`→`tf_states`, `joints`→`joint_states`,
  `node_graph`→`node_graph_edges`, `traces`→`otel_traces`, `diagnostics`→`otel_metrics`, etc.
- **PascalCase ClickHouse column convention** (AC-P2, the case-sensitivity fix):
  `trace_id`→`TraceId`, `span_name`→`SpanName`, `status`→`StatusCode`, … — while the
  ROS2-native tables (`tf_states`, `node_graph_edges`, `joint_states`) keep **bare
  snake_case** columns in both schema profiles.
- **Presentation-layer `format_hint`** per query shape, including
  `SHOW NODE GRAPH`→`NodeGraph` and `MESSAGE FLOW`→`DirectedGraph`.

## What the OSS crate actually exposes

The OSS `rosql` crate is a **parser** (ROSQL text → AST) plus `src/drivers/`
conventions. It does **not** emit ClickHouse SQL — that lives in the go_backend
planner, and a future OSS ClickHouse *driver* is the separate **ROB-224**. So the
OSS-side conformance asserts exactly three things, which are all the OSS side owns:

1. `parse(query)` resolves to the correct `DataSource` (→ `data_source_key`).
2. The registry (`otel_registry(SchemaProfile::OtelClickhouse)`) maps that source to
   `expected_table` and maps OTel fields to their canonical PascalCase columns.
3. `infer_format(ast)` yields `expected_format_hint`.

It does **not** assert generated SQL strings (there are none on the OSS side yet).
That half of the contract — "the planned SQL targets `expected_table` and references
these PascalCase columns" — is what **go_backend** asserts against the same file.

## Fixture format (`cases.json`)

Top-level keys:

| key | meaning |
|-----|---------|
| `$schema_version` | integer; bump on breaking format changes |
| `schema_profile` | `"OtelClickhouse"` or `"OtelPostgres"` — which column profile the column expectations are written against |
| `column_cases[]` | canonical column-identifier assertions |
| `query_cases[]` | per-query source/table/format-hint assertions |

### `column_cases[]`

Two shapes (a case uses exactly one):

```jsonc
// single-table
{ "name": "...", "table": "otel_traces",
  "expected_columns": { "trace_id": "TraceId", "span_name": "SpanName", ... } }

// multi-table
{ "name": "...",
  "tables": { "joint_states": { "effort": "effort", ... }, "tf_states": { ... } } }
```

Each `{ field: column }` entry means: for `schema_profile`, the ROSQL field `field`
on that table must map to the database column `column`.

### `query_cases[]`

```jsonc
{
  "name": "tf_maps_to_tf_states",
  "query": "FROM tf",
  "data_source_key": "tf",        // null for compound queries (TRACE, SHOW NODE GRAPH, MESSAGE FLOW)
  "expected_table": "tf_states",  // null when data_source_key is null
  "expected_format_hint": "Table" // Debug name of rosql::ast::FormatHint
}
```

`expected_format_hint` values are the `FormatHint` enum's Debug names:
`Table`, `LineChart`, `StackedLineChart`, `BarChart`, `HorizontalBars`, `Gantt`,
`DirectedGraph`, `NodeGraph`, `ScalarCards`, `LogTable`, `RecordingList`.

Any `*_comment` / `comment` fields are documentation-only and ignored by the loader.

## How go_backend consumes the same cases (parity follow-up)

The go_backend-side test (a **separate change**) loads this exact file (vendored or
git-submoduled from the rosql repo) and, for each `query_case`:

1. Plans `query` through the ClickHouse planner.
2. Asserts the generated SQL's primary table == `expected_table`.
3. For OTel sources, asserts the SQL references the PascalCase columns from the
   matching `column_cases` entry (and that native-table columns stay snake_case).
4. Optionally asserts the planner's emitted `format_hint` == `expected_format_hint`.

Because both sides read one file, a change to a table mapping or a column convention
must be made in `cases.json` and will fail **both** suites until both implementations
agree — that's the anti-drift guarantee.

## Running

```sh
cargo test --test conformance      # OSS side
```

Wired into CI via the default `cargo test` job (`.github/workflows/ci.yml`).
