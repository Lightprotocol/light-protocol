// ============================================================================
// DECOMPRESS TESTS (compressed → Solana account)
// ============================================================================
//
// Sum Check Failures:
// 1. amount more than output (should fail with output sum check)
// 2. amount less than output (should fail with input sum check)
//
// Authority Field Validation:
// 3. authority != 0 (MUST be 0 for decompress mode) → InvalidInstructionData
//   NOTE: Decompress doesn't use authority field, it must always be 0
//
// Input Out of Bounds:
// 4.1. mint out of bounds
// 4.2. recipient out of bounds
//
// SPL Token Decompression Pool Validation:
// 5. spl token decompression
//  5.1 invalid pool account (invalid derivation seed, valid pool index, valid bump)
//  5.2 invalid pool account (valid derivation seed, valid pool index, invalid bump)
//  5.3 invalid pool account (valid derivation seed, invalid pool index, valid bump)
//  5.4 pool account out of bounds
//  5.5 pool index 6 (higher than max 5)
//
// has_delegate Flag Mismatch:
// 6.1. Input: has_delegate=true but delegate=0
// 6.2. Input: has_delegate=false but delegate!=0
// 6.3. Output: has_delegate=true but delegate=0
// 6.4. Output: has_delegate=false but delegate!=0
//
