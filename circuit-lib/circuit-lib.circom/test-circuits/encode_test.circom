pragma circom 2.1.2;

include "../src/elgamal-babyjubjub/encryot.circom";

component main { public [ ciphertext ] } = Encode();