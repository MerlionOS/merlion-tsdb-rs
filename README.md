# merlion-tsdb-rs

Modern Rust reimplementation of the [Prometheus](https://github.com/prometheus/prometheus) TSDB storage engine.

> **Status: scaffold only.** Module structure laid out; no functional code yet. Implementation is being done from [SPEC.md](SPEC.md) with the upstream Go source as a secondary reference. See [`merlion-tsdb-cpp`](https://github.com/MerlionOS/merlion-tsdb-cpp) for the parallel C++ implementation.

## Goals

- **Wire-format compatible** with Prometheus v3.x blocks, WAL segments, and chunk files — drop-in for `/data` directories produced by upstream Go Prometheus or by `merlion-tsdb-cpp`.
- **Idiomatic modern Rust**: 2024 edition, `Result<T, E>` (no `unwrap` outside tests), `#[deny(unsafe_op_in_unsafe_fn)]`, zero-copy where the format allows.
- **Cross-validated** against the C++ port and against Go-produced golden vectors. The validation matrix:

  | Producer | Consumer | Test |
  |---|---|---|
  | Go    | Rust  | replay upstream `testdata/` blocks |
  | Rust  | Go    | tooling can read what we write |
  | Rust  | C++   | shared on-disk format ⇒ binary diff = 0 |
  | C++   | Rust  | same |

## Scope (initial)

| Component | Upstream Go | SPEC § | Status |
|---|---|---|---|
| `bstream` (bit I/O) | `tsdb/chunkenc/bstream.go` | §2.3 | spec'd, awaiting impl |
| `varint` (LEB128 + zigzag) | `encoding/binary` | §2.2 | spec'd, awaiting impl |
| XOR / Gorilla chunk | `tsdb/chunkenc/xor.go` | §3.1 | spec'd, awaiting impl |
| WAL reader/writer | `tsdb/wlog/` | §4 (TBD) | skeleton |
| Head block (in-memory) | `tsdb/head.go` | §5 (TBD) | skeleton |
| Persistent block + index v3 | `tsdb/block.go`, `tsdb/index/` | §6 (TBD) | skeleton |

Out of scope (separate future projects): PromQL engine, scrape loop, service discovery, remote write/read, web UI.

## Build

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
```

Requires Rust 1.85+ (edition 2024).

## Layout

```
src/
  lib.rs
  encoding/    bstream, varint  — primitives
  chunkenc/    XOR (Gorilla) and histogram encoders
  wal/         Write-ahead log
  head/        In-memory head block
  block/       Persistent block (meta.json, index v3, chunks/, tombstones)
tests/         Integration tests (read golden testdata/, cross-validate)
SPEC.md        On-disk format specification — single source of truth
```

## How implementations stay in sync

There are three reference points:

1. **SPEC.md** — the contract. Byte-level layouts, invariants, error categories. Update **before** touching code.
2. **`../prometheus/tsdb/`** — upstream Go source. The original implementation. Use as a behavioural reference.
3. **`../merlion-tsdb-cpp/`** — parallel C++ port. Cross-check edge cases.

Any wire-format disagreement between any two of the three points should block until the spec is corrected.

## License

[Apache License 2.0](LICENSE) — same as upstream Prometheus. See [NOTICE](NOTICE) for attribution.
