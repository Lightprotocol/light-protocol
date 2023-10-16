pragma circom 2.1.4;
include "encrypt.circom";
template init_vote() {
    signal input publicElGamalPublicKeyX;
    signal input publicElGamalPublicKeyY;
    signal input publicZeroYesEmphemeralKeyX;
    signal input publicZeroYesEmphemeralKeyY;
    signal input publicZeroYesCiphertextX;
    signal input publicZeroYesCiphertextY;

    signal input nonce;

    component encryptZero = Encrypt();
    encryptZero.message <== [0,1];
    encryptZero.publicKey <== [publicElGamalPublicKeyX, publicElGamalPublicKeyY];
    encryptZero.nonce <== nonce;
    publicZeroYesEmphemeralKeyX === encryptZero.ephemeralKey[0];
    publicZeroYesEmphemeralKeyY === encryptZero.ephemeralKey[1];
    publicZeroYesCiphertextX === encryptZero.ciphertext[0];
    publicZeroYesCiphertextY === encryptZero.ciphertext[1];
}