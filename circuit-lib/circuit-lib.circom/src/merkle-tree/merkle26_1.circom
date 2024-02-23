pragma circom 2.1.4;
include "./MerkleTreeProof.circom";

component main {public [root, leaf]} = MerkleTreeProof(26, 1);
