// Package masp benchmarks a Multi-Asset Shielded Pool gnark circuit.
//
// See README.md for overview, constraint counts, and the split between
// utxo_circuit (P-256 verify + UTXO/nullifier/hashchain hashing) and
// tree_circuit (Merkle inclusion + indexed-tree non-inclusion).
package masp
