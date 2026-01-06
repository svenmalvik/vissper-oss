# Rust Core Best Practices

## Tooling and Style
- Use current stable toolchain; run `cargo fmt` and `cargo clippy --all-targets --all-features` before committing
- Prefer `#![deny(clippy::all)]` at crate root; opt into pedantic groups only when helpful
- Keep modules small and cohesive; favor explicit `pub(crate)` over `pub` to limit surface area
- Keep files to around 300 lines of code; extract submodules when files grow larger
- Document public items with `///` and include minimal examples that compile as doctests

## Ownership and Data Structures
- Borrow by default; avoid `clone()` unless profiling or clarity shows it is needed
- Prefer `&[T]`/`&str` over `Vec<T>`/`String` in APIs to avoid allocations
- Return owned data when ownership transfer is intended
- Use `Cow` when callers may pass borrowed or owned data
- Use `Arc` for shared ownership across threads
- Keep mutability narrow; prefer building immutable structs via builders, then expose read-only accessors
- Use `Option`/`Result` instead of sentinel values; use `NonZero*` types when zero is invalid

## API Design
- Minimize type and lifetime complexity in public APIs
- Accept trait bounds only as needed
- Prefer `From`/`TryFrom` conversions over custom constructors where it fits
- Use newtypes to add meaning to primitives
- When configuration grows, use the builder pattern and make defaults explicit
- Iterate over slices/iterators instead of indexing collections
- Return iterators where possible to avoid allocations

## Testing
- Co-locate unit tests in modules (`mod tests { ... }`)
- Keep integration tests in `tests/`
- Use table-driven tests for coverage
- Prefer `assert_eq!`/`assert_matches!` with clear messages
- Add property-based tests (e.g., `proptest`) for invariants
- Benchmark with `cargo bench` when performance matters

## Performance
- Measure before optimizing; use `cargo flamegraph`/`perf`/`dhat` when needed
- Prefer iterators over intermediate `Vec`s; avoid needless `collect`
- Use `&str` over `String` for parsing
- Reuse buffers and prefer small-copy types (`Copy`, `SmallVec`, `arrayvec`) when profiles justify
- Keep `unsafe` code isolated, well-documented, and covered by tests

## Packaging
- Use workspaces for multiple crates
- Keep shared lint/configuration in workspace `Cargo.toml`
- Expose binaries via `src/bin/*.rs` to keep `main.rs` lean
- Move logic into libraries for testability
- Keep feature flags additive and documented
