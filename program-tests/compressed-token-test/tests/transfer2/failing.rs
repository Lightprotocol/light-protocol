// Tests:
// Transfer:
// 1. invalid owner has signed
// 2. owner is valid but not signer
// 3. unbalanced transfer (too little inputs) (should fail with input sum check)
// 4. unbalanced transfer (too little outputs) (should fail with output sum check)
// 5. invalid delegate has signed
// 6. delegate is valid but not signer (owner hasn't signed either)
// 7. invalid mint (should fail with 14137 invalid hash)
// 8. invalid version (should fail with 14137 invalid hash)
// 9. Input out of bounds errors:
//  9.1. owner out of bounds
//  9.2. delegate out of bounds
//  9.3. mint out of bounds
// 10. Output out of bounds errors:
//  10.1. owner out of bounds
//  10.2. delegate out of bounds
//  10.3. mint out of bounds
//
// Compress:
// 1. amount more than output (should fail with output sum check)
// 2. amount less than output (should fail with input sum check)
// 3. ctoken compression
//  3.1 invalid authority has signed
//  3.2 authority is valid but not signer
// 4. spl token compression
//  4.1 invalid pool account (invalid derivation seed, valid pool index, valid bump)
//  4.2 invalid pool account (valid derivation seed, valid pool index, invalid bump)
//  4.3 invalid pool account (valid derivation seed, invalid pool index, valid bump)
//  4.4. pool account out of bounds
//  4.5. pool index 6 (hihher than max 5)
// 5. Output out of bounds errors:
//  5.1. authority out of bounds
//  5.2. mint out of bounds
//  5.3. recipient out of bounds
//
// TODO: CompressAndClose later
//
// Decompress:
// 1. amount more than output (should fail with output sum check)
// 2. amount less than output (should fail with input sum check)
// 3. invalid authority has signed
// 4. authority is valid but not signer
// 5. Output out of bounds errors:
//  5.1. authority out of bounds
//  5.2. mint out of bounds
//  5.3. recipient out of bounds
// 6. spl token decompression
//  6.1 invalid pool account (invalid derivation seed, valid pool index, valid bump)
//  6.2 invalid pool account (valid derivation seed, valid pool index, invalid bump)
//  6.3 invalid pool account (valid derivation seed, invalid pool index, valid bump)
//  6.4. pool account out of bounds
//  6.5. pool index 6 (hihher than max 5)
//
//
// Test setup for Transfer:
// 1. create and mint to one compressed token account (undelegated)
//  - add option to delegate the entire balance
// Test setup for Compress ctoken:
// 1. create and mint to ctoken compressed account
//
// Test setup for Compress spl token:
// 1. create spl token mint and mint to spl token account
