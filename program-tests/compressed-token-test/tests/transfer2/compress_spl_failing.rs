// SPL Token Compression Pool Validation:
// 4. spl token compression
//  4.1 invalid pool account (invalid derivation seed, valid pool index, valid bump)
//  4.2 invalid pool account (valid derivation seed, valid pool index, invalid bump)
//  4.3 invalid pool account (valid derivation seed, invalid pool index, valid bump)
//  4.4 pool account out of bounds
//  4.5 pool index 6 (higher than max 5)

// Test setup for Compress spl token:
// 1. create spl token mint and mint to one spl token account
