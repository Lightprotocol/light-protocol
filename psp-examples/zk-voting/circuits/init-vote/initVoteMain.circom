pragma circom 2.1.4;
include "./init_vote.circom";

component main { public [
    publicElGamalPublicKeyX,
    publicElGamalPublicKeyY,
    publicZeroYesEmphemeralKeyX,
    publicZeroYesEmphemeralKeyY,
    publicZeroYesCiphertextX,
    publicZeroYesCiphertextY
]} =  init_vote();