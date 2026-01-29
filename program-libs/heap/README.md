<!-- cargo-rdme start -->

# light-heap

Custom bump allocator for Solana programs. Replaces the default
global allocator to allow explicit cursor control and heap freeing.

| Type | Description |
|------|-------------|
| [`BumpAllocator`] | Global bump allocator with `alloc` and `dealloc` |
| [`HeapError`] | Invalid heap position error |
| [`bench`] | Heap usage benchmarking utilities |

<!-- cargo-rdme end -->
