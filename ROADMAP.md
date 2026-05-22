# merlion-tsdb-rs Roadmap

This file tracks implementation progress against `SPEC.md`. Only sections marked
Final or Drafted should be implemented; TBD sections wait for the spec to land.

## Current PR

- PR: <https://github.com/MerlionOS/merlion-tsdb-rs/pull/1>
- Branch: `codex/implement-encoding-xor`
- Agent: Codex

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
| §4 WAL | `wal` | Blocked | SPEC section is TBD. |
| §5 Head block | `head` | Blocked | SPEC section is TBD. |
| §6 Persistent block | `block` | Blocked | SPEC section is TBD. |
| §7 Tombstones | TBD | Blocked | SPEC section is TBD. |

## Validation

Current PR validation:

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets -- -D warnings`

## Workflow Notes

- Use separate git worktrees and PR branches for new work to avoid conflicts
  with Claude Code and Codex CLI.
- Commits authored by Codex should use `Codex <codex@openai.com>`.
