# ADR-0029: Contract-Declared Structs Materialize as Real Classes

## Status

Accepted. Implemented: `TimeParts`, `RawStats`, `WsFrame`, `HttpRequestHead`,
`UiEvent`, `TypedText`, `ShapedRun` are contract structs with frozen layouts,
built natively and read through fixed slots. Supersedes the positional-tuple
FFI wire used as an interim step (tuples for `time`/`fs`/`ws`/`net` removed in
the same effort, not kept as a parallel path).

## Context

Every value crossing the native boundary arrived as a plain object typed as an
interface. The checker knew the precise member types, but codegen had no slot
to publish, so each read fell through to dynamic name dispatch: 66 of the 273
non-module generics in `std` lived on this boundary (`msToParts$`, `stat$`,
`wsParseFrame$`, `parseHttpRequest$`, `poll$`/`typed$`, `shape$`).

The macro system could not fix it despite having all the pieces. `vn_record!`
generated record builders only from `interface` contracts (plain objects);
its `class:` source generated `alloc_instance` against a registry that only
contains intrinsics (`vn-vm/src/exec/host/mod.rs:370`), so no std shape could
ever resolve. `natives! class` built methods but never data layouts with value
semantics, and the contract language could not even name the thing that was
missing: `vn-contract` parses `class` and `interface`, and a struct only
worked by accident of parsing as `Decl::Class`.

The interim fix (positional tuples over `alloc_tuple`) made everything static
but scaled badly by construction: field order lived in three places (Rust
vec, declaration comment, Varn destructure), with silent misreads as the
failure mode for every future native.

## Introspección comparada (Ley 21)

- **C# (P/Invoke)**: blittable structs — the same field declaration on both
  sides of the boundary produces the same layout, no marshaling layer. This
  is the adopted model: one declaration (the contract), two materializations
  (checker `ClassInfo`, runtime `ClassObj`).
- **Rust**: `#[repr(C)]` exists for the same reason (layout as contract), and
  the macro here plays that role, generating both sides from one source.
- **Swift**: value types crossing the ObjC boundary need explicit bridging;
  without it they stay dynamic. The lesson is that the boundary must be
  designed, not inherited — hence registration at module init rather than
  lookup by accident.
- **Kotlin/Dart (JNI/FFI)**: cross-boundary values stay boxed/dynamic unless
  declared blittable. Keeping `drawn$`/`shape$` consumers as dynamic
  interfaces follows the same line: values that escape to user code keep the
  honest dynamic boundary; values consumed structurally inside `std` get
  layouts.
- **TypeScript (puente)**: nothing changes on the surface. `new TimeParts({
  year, ... })`, `parts.year`, `==` by value all read like the TS shape they
  mirror; only the representation behind them is now static. No new syntax
  was invented: `export struct` in contracts is the same declaration Varn
  code already uses.

## Decision

**A struct declared in a runtime contract materializes as a real frozen class
with value semantics, buildable from Rust and `new`-able from Varn, with
member reads lowering to fixed slots on both sides.**

### 1. The contract is the single source of truth

`export struct` in a `.vn` contract parses as `Decl::Class` with
`is_struct: true` (pre-existing parser behavior). `vn-contract` now carries
`ClassContract.is_struct` (`crates/vn-contract/src/model.rs:35`) and expands
the synthesized constructor params into `Property` members, so layout,
builder, and record generator all read the same field list in the same order.

### 2. The native builder registers a value class

`natives! { class X {} }` needs no hand-written members for a data shape: the
builder extends the layout from the contract properties, marks it value
(`ClassObj::mark_value`, `crates/vn-types/src/value/class.rs:125`), freezes,
and registers it (`crates/vn-macros/src/natives/mod.rs:290`). A user-written
constructor for a struct is a compile error, mirroring Varn ("a struct has no
constructor: its fields, in order, are its constructor"); the macro
synthesizes `constructor(ctx, this, f0, ...)` which sets each field and
returns `this`, so `new X(...)` works positionally and by name.

### 3. Construction on both sides goes through one layout

Rust builds instances with `vn_record!` (`class:` source, same contract) plus
`alloc_instance`; Varn builds them with synthesized `new`. Both resolve the
same registered class, so slots computed by the compiler match the runtime
layout by construction. `==` compares by value through the existing
`layout.is_value` composite path.

### 4. Ownership rule

The shape lives in the contract of the module that produces it
(`runtime:time` owns `TimeParts`); `std` imports and re-exports the type and
keeps the behavior. Shapes built in Varn (`DurationParts`, `CronSpec`,
`CacheEntry`) stay `std` structs; nothing moves. Natives whose values escape
to user code or deliberate dynamic test coverage (`drawn$`, `shape$`
structure) keep plain-object returns.

### Deleted

- The positional-tuple wire (`alloc_tuple` returns, tuple-typed declares,
  destructuring call sites, wire-order comments) for `time`, `fs`, `ws`,
  `net`. No tuple path survives alongside the struct path.
- The `RawStats`/`WsFrame`/`HttpRequestHead` interfaces and their
  `vn_record` interface-source bridges.
- The `<module>`-level re-export noise is untouched (separate machinery).

### Kept / strengthened

- `vn_record!` interface source for genuinely dynamic shapes (`DrawnItem`).
- `alloc_instance` name resolution (intrinsics table); qualified names are a
  hardening follow-up, not a redesign: collisions are impossible today
  because each shape name is unique across runtime contracts.
- `==` on structs stays dynamically dispatched (pre-existing for all
  structs); only its semantics are now correct by construction.

## Verification

- `vn debug -p typeloss` is zero non-module generics on every touched file
  (`time/*`, `fs/*`, `ws/*`, `request`, `ui/*`, `text`).
- `tests/main.vn` 6155/0 in JIT and `VN_NO_JIT=1`, `verify.ps1 -Fast` clean
  before each push; `cargo clippy` and `cargo fmt` clean.
- `tests/321` locks value semantics (`toParts(ms) == toParts(ms)`); time
  roundtrips cover field order (a layout mismatch fails loudly, not silently).
- The macro enforces the contract at compile time: an unbound contract
  member or a hand-written struct constructor fails the build, so drift
  between declaration, layout, and constructor is impossible.

## Alternatives rejected

- **Permanent tuples**: worked and green, but the order hazard grows with
  every native. Explicitly a patch, forbidden as an endpoint by Ley 8.
- **V8-style hidden classes for all objects**: would also fix structural
  types in general, but it is a VM-wide object-model project to solve an FFI
  boundary problem. Disproportionate.
- **Registering std structs as intrinsics**: layering inversion; the
  producer module must own its shapes.
- **Hand-written constructors per shape**: linear boilerplate for every
  future native. The synthesized constructor is the scalability fix the
  tuple wire lacked.
