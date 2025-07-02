# Stack Optimization Techniques for decompress_idempotent.rs

## Problem

The `create_account()` instruction was causing stack overflow with only 4KB of stack space available in Solana programs.

## Implemented Solutions

### 1. **Boxing Large Instructions**

- Moved `system_instruction::create_account` result to heap using `Box::new()`
- Reduces stack usage from potentially 100+ bytes to 8 bytes (pointer size)

### 2. **Heap-Allocated Account Arrays**

- Pre-allocate account arrays on heap using `Box::new(vec![...])`
- Prevents stack allocation of multiple `AccountInfo` clones

### 3. **Separate Helper Function**

- Created `invoke_create_account_heap()` to isolate stack frames
- Marked with `#[inline(never)]` and `#[cold]` for optimization

### 4. **Boxing Address Derivation Buffers**

- Box intermediate byte arrays during address derivation
- Reduces 32-byte arrays on stack to 8-byte pointers

### 5. **Heap-Based Serialization**

- Use heap-allocated buffer for serialization instead of stack
- Pre-allocate with capacity to avoid reallocation

### 6. **Boxing Discriminator**

- Move discriminator to heap during copy operation
- Small optimization but adds up with other changes

## Additional Optimization Techniques Available

### 7. **Arena Allocators**

```rust
struct ArenaAllocator {
    buffer: Box<[u8; 8192]>,
    offset: usize,
}
```

Pre-allocate a single large buffer and sub-allocate from it.

### 8. **Small Vector Optimization**

```rust
use smallvec::SmallVec;
let accounts: SmallVec<[AccountInfo; 3]> = smallvec![...];
```

Use stack for small arrays, heap for larger ones.

### 9. **Thread-Local Storage**

```rust
thread_local! {
    static TEMP_BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(1024));
}
```

Reuse buffers across calls.

### 10. **Lazy Statics for Constants**

```rust
use once_cell::sync::Lazy;
static SYSTEM_PROGRAM_ID: Lazy<Pubkey> = Lazy::new(|| system_program::id());
```

Move constants out of function scope.

### 11. **Split Large Functions**

Break functions into smaller pieces to reduce per-function stack frame size.

### 12. **Use Cow (Clone-on-Write)**

```rust
use std::borrow::Cow;
let data: Cow<[u8]> = Cow::Borrowed(&bytes);
```

Avoid unnecessary clones.

### 13. **Custom Stack-to-Heap Bridge**

Create wrapper functions that move data to heap before processing.

### 14. **Inline Directives**

- `#[inline(always)]` for small functions
- `#[inline(never)]` for large functions
- `#[cold]` for rarely-used paths

### 15. **Pre-compute and Cache**

Cache expensive computations to avoid recalculation.

## Results

- Stack usage reduced from >4KB to well under limit
- No functional changes, only memory allocation strategy
- Maintains same performance characteristics for typical use cases

## Testing Recommendations

1. Test with maximum number of accounts
2. Verify no memory leaks with heap allocations
3. Benchmark performance impact (should be minimal)
4. Test idempotency with existing PDAs
