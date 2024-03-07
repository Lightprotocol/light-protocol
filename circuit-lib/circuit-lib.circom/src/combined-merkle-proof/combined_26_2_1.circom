pragma circom 2.1.4;
include "./CombinedMerkleProof.circom";

component main {public [root, leaf, niRoot, niValue]} = CombinedMerkleProof(26, 2, 1);
