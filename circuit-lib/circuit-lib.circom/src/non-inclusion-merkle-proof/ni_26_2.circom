pragma circom 2.1.4;
include "./NonInclusionMerkleProof.circom";

component main {public [root, value]} = NonInclusionMerkleProof(26, 2);
