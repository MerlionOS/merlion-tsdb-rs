# merlion-tsdb-rs Roadmap

This file tracks implementation progress against `SPEC.md`. Only sections
marked **Final** or **Drafted** should be implemented; TBD sections wait
for the spec to land.

## Progress

| SPEC section | Module | Status | Notes |
|---|---|---|---|
| §2.1 Endianness | shared convention | Done | Big-endian convention applied where relevant. |
| §2.2 Varint | `encoding::varint` | Done | Go-compatible uvarint and zigzag varint with unit tests. |
| §2.3 Bit stream | `encoding::bstream` | Done | MSB-first writer/reader, varint-through-bitstream, boundary tests. |
| §3.1 XOR / Gorilla | `chunkenc::xor` | Done | Float sample append and iteration implemented with unit tests. |
| §3.2 XOR2 | `chunkenc` | Blocked | SPEC section is TBD. |
| §3.3 Histogram | `chunkenc` | Blocked | SPEC section is TBD. |
| §3.4 Float histogram | `chunkenc` | Blocked | SPEC section is TBD. |
| §4 WAL | `wal` | **Ready** | SPEC promoted to Final on 2026-05-22. Mirror C++ phases 1–5 below. |
| §5 Head | `head` | **Ready** | SPEC promoted to Final. Depends on §4. |
| §6 Persistent block | `block` | **Ready** | SPEC promoted to Final. Depends on §3.1 (already done) + §6.7 ULID. |
| §6.8 Tombstones | `block` | Drafted | MVP writes empty file; non-empty format deferred. |
| §7 Compaction | `block` | **Ready** | New section. Depends on §6 read + write. |

## Next-up phases (suggested PR slicing)

These mirror the C++ port's PR boundaries so each Rust slice has a
ready-made reference implementation + test suite at the corresponding
path in `../merlion-tsdb-cpp/`.

### §4 WAL (5 PRs, ~80 tests on the C++ side)

1. **CRC32-Castagnoli** primitive (`encoding::crc32c`) — mirrors
   `merlion-tsdb-cpp/src/encoding/crc32c.cpp`. Foundational; the
   following WAL phases all use it.
2. **Page + record framing** (`wal::page`) — 32 KiB page format, 7-byte
   record header (type | u16-BE length | u32-BE CRC), fragmenting
   protocol. In-memory writer + reader; no file I/O yet.
3. **Segment writer** (`wal::segment_writer`) — file I/O, 128 MiB
   rollover, fsync semantics.
4. **Segment reader** (`wal::segment_reader`) — multi-segment iteration,
   torn-tail tolerance on last segment.
5. **Series + SamplesV2 record codecs** (`wal::record`) — type-1 and
   type-11 record bodies per SPEC §4.7.

### §5 Head (4 PRs)

1. **`model::Labels`** — sorted/deduped label set, hash, lookup.
2. **`MemSeries` + `SeriesStore`** — in-memory state, ref allocation,
   chunk lifecycle.
3. **`Head::append` + WAL integration** — emits Series + SamplesV2
   records on commit.
4. **WAL replay on `Head::open`** — closes the durable round-trip.

### §6 Persistent block (5 PRs)

1. **`block::meta`** — meta.json read/write via `serde_json`.
2. **`block::chunks`** — segment file read/write, `BlockChunkRef`
   encoding (0-indexed seq).
3. **`block::index` reader** — header + TOC + symbol table + postings
   + series.
4. **`block::index_writer`** — V2 emission with 16-byte series
   alignment.
5. **`block::Block`** — orchestrator (`open`, `query`,
   `create_from_series`, `create_from_head`).

### §7 Compaction (1 PR)

Mirrors `merlion-tsdb-cpp/src/block/block.cpp::Block::compact`.

## Cross-validation milestones

Once both sides exist for any layer, exchange-test by handing a
C++-produced file to the Rust reader and vice versa. Concrete targets:

- **Layer 1 (chunks)**: write an XOR chunk in Rust, read in C++. Same
  reverse direction.
- **Layer 2 (WAL)**: write a WAL with a `Series` record + a
  `SamplesV2` record in Rust, replay in C++.
- **Layer 3 (block)**: `merlion-tsdb-cpp::Block::create_from_series`
  → Rust's `block::Block::open` recovers same series. Bidirectional.

The `merlion-tsdb-cpp/testdata/index_format_v1/` fixture (upstream's
golden block) is the ultimate cross-check: every Rust subsystem must
ingest it once §6 lands.

## Validation

Per-PR CI:

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`

## Workflow Notes

- Use separate git worktrees and PR branches for new work to avoid
  conflicts with Claude Code and Codex CLI sessions on this repo.
- Commits authored by Codex should use `Codex <codex@openai.com>`.
- Spec changes land in lock-step in both `merlion-tsdb-cpp` and
  `merlion-tsdb-rs` — see the contract in SPEC.md §1.
