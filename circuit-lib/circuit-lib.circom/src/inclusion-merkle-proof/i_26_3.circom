pragma circom 2.1.4;
include "./InclusionMerkleProof.circom";

component main {public [root, leaf]} = InclusionMerkleProof(26, 3);
