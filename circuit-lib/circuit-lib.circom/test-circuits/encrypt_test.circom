pragma circom 2.1.2;

include "../src/elgamal-babyjubjub/encrypt.circom";

component main { public [ publicKey ] } = Encrypt();