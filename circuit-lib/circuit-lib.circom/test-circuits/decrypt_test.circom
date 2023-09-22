pragma circom 2.1.2;

include "../src/elgamal-babyjubjub/decrypt.circom";

component main { public [ ciphertext ] } = Decrypt();