<!-- cargo-rdme start -->

# light-indexed-array

Indexed array for indexed Merkle trees. Stores elements as
a sorted linked list with index, value, and next-index pointers.

| Type | Description |
|------|-------------|
| [`array::IndexedElement`] | Element with index, BigUint value, and next-index |
| [`array::IndexedArray`] | Array of indexed elements with insert and lookup |
| [`changelog`] | Raw indexed element and changelog entry types |
| [`errors`] | `IndexedArrayError` variants |

<!-- cargo-rdme end -->
