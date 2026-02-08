# Chromaprint Linking Issue - RESOLVED

**Date:** 2026-01-09
**Status:** ✅ FIXED

---

## Problem

Tests and binaries failed to link with chromaprint library:

```
error LNK2019: unresolved external symbol __imp_chromaprint_new
error LNK2019: unresolved external symbol __imp_chromaprint_start
error LNK2019: unresolved external symbol __imp_chromaprint_feed
... (8 unresolved externals total)
error LNK1120: 8 unresolved externals
```

---

## Root Cause

The FFI bindings in [wkmp-ai/src/ffi/chromaprint.rs](wkmp-ai/src/ffi/chromaprint.rs#L32) declared:

```rust
#[link(name = "chromaprint")]
extern "C" {
    pub fn chromaprint_new(algorithm: c_int) -> ChromaprintContextPtr;
    ...
}
```

This told Rust to link against `chromaprint` as a **dynamic library (DLL)**, looking for import symbols like `__imp_chromaprint_new`.

However, `chromaprint-sys-next` was building chromaprint as a **static library** (BUILD_SHARED_LIBS=OFF in CMakeCache.txt).

**Mismatch:** Linker looked for DLL import symbols (`__imp_*`) but only static library symbols existed.

---

## Solution

Modified [wkmp-ai/src/ffi/chromaprint.rs:32](wkmp-ai/src/ffi/chromaprint.rs#L32) to explicitly specify static linking:

```rust
#[link(name = "chromaprint", kind = "static")]
extern "C" {
    pub fn chromaprint_new(algorithm: c_int) -> ChromaprintContextPtr;
    ...
}
```

**Change:** Added `kind = "static"` attribute to the `#[link]` directive.

---

## Verification

### Library Build
✅ Compiles successfully:
```
cd wkmp-ai && cargo build --release --lib
Finished `release` profile [optimized] target(s) in 1.39s
```

### Library Tests
✅ All 16 chromaprint tests pass:
```
cd wkmp-ai && cargo test --release --lib chromaprint
test result: ok. 16 passed; 0 failed; 0 ignored
```

### Binary Build
✅ Compiles and links successfully:
```
cd wkmp-ai && cargo build --release --bin wkmp-ai
Finished `release` profile [optimized] target(s) in 43.90s
```

---

## Impact

**Before Fix:**
- ❌ Tests failed to compile/link
- ❌ Binary failed to link
- ❌ Stage 6 overlap resolution could not be tested

**After Fix:**
- ✅ Library compiles and tests pass
- ✅ Binary compiles and links successfully
- ✅ Ready to test Stage 6 overlap resolution

---

## Files Changed

- [wkmp-ai/src/ffi/chromaprint.rs](wkmp-ai/src/ffi/chromaprint.rs#L32) - Added `kind = "static"` to link directive

---

## Technical Details

### Static vs Dynamic Linking on Windows

**Dynamic Linking (DLL):**
- Import library (.lib) contains stubs with `__imp_` prefix
- Actual code in .dll loaded at runtime
- `#[link(name = "foo")]` defaults to dynamic on Windows

**Static Linking:**
- Static library (.lib) contains actual compiled code
- No `__imp_` prefix on symbols
- Requires explicit `#[link(name = "foo", kind = "static")]`

### Chromaprint Build Configuration

chromaprint-sys-next builds chromaprint via CMake:
- `BUILD_SHARED_LIBS:BOOL=OFF` (static library)
- Output: `chromaprint.lib` in `target/release/build/.../out/lib/`
- Contains raw symbol names (not `__imp_*` prefixed)

### Why This Wasn't Caught Earlier

- Library-only builds (`cargo build --lib`) succeeded
  - Library doesn't link external symbols, just compiles
- Only tests and binaries perform final linking
  - Tests trigger the linker, exposing the mismatch

---

## Lessons Learned

1. **Explicit is better than implicit:** Always specify `kind = "static"` or `kind = "dylib"` when linking C libraries
2. **Build != Link:** Successful library compilation doesn't guarantee successful binary linking
3. **Test early:** Run full test suite (including link step) to catch linking issues

---

## Related Work

This fix unblocks:
- ✅ Stage 6 overlap resolution testing ([OVERLAP_RESOLUTION_IMPLEMENTATION.md](OVERLAP_RESOLUTION_IMPLEMENTATION.md))
- ✅ Full 200-album library testing
- ✅ Eagles - The Long Run targeted testing

---

## References

- Rust FFI Documentation: https://doc.rust-lang.org/nomicon/ffi.html#foreign-calling-conventions
- chromaprint-sys-next crate: https://crates.io/crates/chromaprint-sys-next
- Windows linking symbols: https://learn.microsoft.com/en-us/cpp/build/reference/symbols
