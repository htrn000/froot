# Solver profiling findings

This note captures the first profiling pass over `fruitbox_bench`, plus the
workflow that is now wired into the benchmark CLI for quick iteration.

## How to run profiling now

Generate per-approach flamegraphs and candidate telemetry:

```bash
cargo run --release --bin fruitbox_bench -- \
  --generator fungster \
  --samples 1 \
  --max-states 5000 \
  --flamegraph-dir /tmp/fruitbox-flamegraphs \
  --candidate-profile
```

Run without instrumentation code paths at compile time:

```bash
cargo run --release --bin fruitbox_bench --features no_instrument -- \
  --generator fungster \
  --samples 1 \
  --max-states 5000 \
  --flamegraph-dir /tmp/fruitbox-flamegraphs \
  --candidate-profile
```

When `no_instrument` is enabled, the benchmark logs:

- `event=instrumentation_disabled reason=compiled_with_no_instrument`
- `event=flamegraph_skipped reason=compiled_with_no_instrument`

and does not emit candidate profile rows.

## What we found

### 1) `ordered_candidates` dominates runtime

`gprof` runs over benchmark samples consistently showed
`_native::solver::ordered_candidates` as the primary hot path:

- fungster sample: ~89.62% in `ordered_candidates`;
- random sample: ~96.27% in `ordered_candidates`;
- rejection attempt loops: ~96.48% in `ordered_candidates`.

### 2) Sorting itself is not the main cost

At first glance this can look like sorting overhead, but direct
collect-vs-sort instrumentation inside `ordered_candidates` shows otherwise.

Representative measurements:

- fungster / `dfs_first_largest` (`max-states=5000`):
  - `collect_time_us=698196`
  - `sort_time_us=4117`
  - sort ~= 0.59% of collect+sort time
- fungster / `dfs_first_smallest`:
  - `collect_time_us=10814`
  - `sort_time_us=339`
  - sort ~= 3.04%
- random / `dfs_first_largest` (`max-states=200000`, earlier run):
  - `collect_time_us=33776342`
  - `sort_time_us=24105`
  - sort ~= 0.07%

This indicates the expensive part is mostly candidate generation/filtering
(`valid_moves` + `live_sum` checks) rather than sorting.

### 3) Candidate vector sizes depend heavily on board family

Observed candidate profile distributions:

- fungster / `dfs_first_largest` (`max-states=5000`):
  - `avg_candidates=101.24`, `p50=95`, `p90=176`, `p99=234`, `max=258`
- fungster / `dfs_first_smallest`:
  - `avg_candidates=229.86`, `p50=235`, `p90=335`, `p99=384`, `max=384`
- random / `dfs_first_largest` (`max-states=200000`, earlier run):
  - `avg_candidates=13.17`, `p50=13`, `p90=21`, `p99=27`, `max=52`

Random boards had smaller candidate vectors on average but still consumed heavy
time because large numbers of states hit the same generation/filtering path.

## Where to optimize first

Given the current profile, optimize this order:

1. reduce `valid_moves` scan cost;
2. reduce repeated `live_sum` work per rectangle/state;
3. revisit ordering/sort only after (1) and (2).

The new profiling stack should make these iterations faster:

- flamegraphs answer "which stack dominates now?";
- candidate profile logs answer "what shape of candidate workload are we seeing?".

## Follow-up: incremental DFS transition state

The DFS witness paths now maintain per-rectangle live sums/counts and exact
sum-10 candidate buckets during backtracking. This removes repeated
`live_sum` scans from `solve_first_empty` and `has_empty_solution`; exhaustive
DP still uses the mask-state transition path.

Post-change smoke measurement:

- fungster / `dfs_first_largest` (`max-states=1000`):
  - `calls=1000`
  - `total_candidates=109366`
  - `collect_time_us=819`
  - `sort_time_us=0`
- fungster / `dfs_first_smallest`:
  - `calls=66`
  - `total_candidates=13991`
  - `collect_time_us=73`
  - `sort_time_us=0`

Thirty-board release benchmarks with `max-states=1000` and
`max-empty-solutions=1` showed that applying the incremental state to every DFS
ordering was the wrong final shape:

| version | fungster wall | random wall |
| --- | ---: | ---: |
| baseline scan | 5.070s | 14.632s |
| fully incremental DFS | 29.752s | 16.003s |
| hybrid | 4.807s | 17.266s |

For the fungster board family, the per-approach data explains why the hybrid is
the best current shape:

| version | approach | ok / 30 | state-limit / 30 | ok elapsed total |
| --- | --- | ---: | ---: | ---: |
| baseline scan | `dfs_first_largest` | 3 | 27 | 70.201ms |
| baseline scan | `dfs_first_smallest` | 30 | 0 | 322.621ms |
| baseline scan | `dp_exhaustive` | 9 | 21 | 37.609ms |
| fully incremental DFS | `dfs_first_largest` | 4 | 26 | 2094.076ms |
| fully incremental DFS | `dfs_first_smallest` | 30 | 0 | 46.803ms |
| fully incremental DFS | `dp_exhaustive` | 9 | 21 | 39.140ms |
| hybrid | `dfs_first_largest` | 3 | 27 | 74.365ms |
| hybrid | `dfs_first_smallest` | 30 | 0 | 45.774ms |
| hybrid | `dp_exhaustive` | 9 | 21 | 37.553ms |

The fully incremental path made candidate collection nearly free, but
largest-first pays too much apply/undo maintenance cost while exploring many
failed branches. The current hybrid keeps the scan path for largest-first and
uses incremental buckets for smallest-first, where constructed fungster boards
typically follow the small-tile solution path.

Post-hybrid candidate profile for a fungster sample:

- `dfs_first_largest` (`max-states=1000`):
  - `calls=1000`
  - `total_candidates=88207`
  - `collect_time_us=134276`
  - `sort_time_us=836`
- `dfs_first_smallest`:
  - `calls=66`
  - `total_candidates=13991`
  - `collect_time_us=79`
  - `sort_time_us=0`

This is only a net win for fungster at the current cap. Random boards still
prefer the scan path in aggregate, so any future optimization should focus on
reducing incremental apply/undo churn or selecting the transition strategy per
board family and ordering.
