pragma circom 2.1.4;
include "decrypt.circom";

template publish_decrypted_tally() {
    signal input publicVoteWeightYesX;
    signal input publicVoteWeightYesY;
    signal input publicVoteWeightYesEmphemeralKeyX;
    signal input publicVoteWeightYesEmphemeralKeyY;
    signal input publicVoteWeightNoX;
    signal input publicVoteWeightNoY;
    signal input publicVoteWeightNoEmphemeralKeyX;
    signal input publicVoteWeightNoEmphemeralKeyY;
    signal input publicYesResult;
    signal input publicNoResult;

    signal input secretKey;
    signal input xhiYes;
    signal input xloYes;

    signal input xhiNo;
    signal input xloNo;


    component decryptYes = Decrypt();
    decryptYes.ciphertext <== [
        publicVoteWeightYesX,
        publicVoteWeightYesY
    ];
    decryptYes.ephemeralKey <== [
        publicVoteWeightYesEmphemeralKeyX,
        publicVoteWeightYesEmphemeralKeyY
    ];
    decryptYes.secretKey <== secretKey;

    component decryptNo = Decrypt();
    decryptNo.ciphertext <== [
        publicVoteWeightNoX,
        publicVoteWeightNoY
    ];
    decryptNo.ephemeralKey <== [
        publicVoteWeightNoEmphemeralKeyX,
        publicVoteWeightNoEmphemeralKeyY
    ];
    decryptNo.secretKey <== secretKey;

    component decodeYes = Decode(32);
    decodeYes.encodedMessage[0] <== decryptYes.decryptedMessage[0];
    decodeYes.encodedMessage[1] <== decryptYes.decryptedMessage[1];
    decodeYes.xhi <== xhiYes;
    decodeYes.xlo <== xloYes;
    publicYesResult === decodeYes.decodedMessage;

    component decodeNo = Decode(32);
    decodeNo.encodedMessage[0] <== decryptNo.decryptedMessage[0];
    decodeNo.encodedMessage[1] <== decryptNo.decryptedMessage[1];
    decodeNo.xhi <== xhiNo;
    decodeNo.xlo <== xloNo;
    publicNoResult === decodeNo.decodedMessage;
}