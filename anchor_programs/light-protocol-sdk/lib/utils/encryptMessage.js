"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.encryptMessage = void 0;
const nacl = require("tweetnacl");
const toUintArray_1 = require("./toUintArray");
const encryptMessage = function (message, nonce, recipientEncryptionKeypair, senderThrowAwayKeypair) {
    console.log(message, nonce, recipientEncryptionKeypair, senderThrowAwayKeypair);
    var ciphertext = nacl.box((0, toUintArray_1.toUintArray)(message), (0, toUintArray_1.toUintArray)(nonce), (0, toUintArray_1.toUintArray)(recipientEncryptionKeypair.publicKey), (0, toUintArray_1.toUintArray)(senderThrowAwayKeypair.secretKey));
    return ciphertext;
};
exports.encryptMessage = encryptMessage;
