"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getExtDataHash = void 0;
const ethers_1 = require("ethers");
const constants_1 = require("../constants");
const getExtDataHash = function (
// inputs are bytes
recipient, extAmount, relayer, fee, merkleTreePubkeyBytes, encryptedOutput1, encryptedOutput2, nonce1, nonce2, senderThrowAwayPubkey1, senderThrowAwayPubkey2) {
    let encodedData = new Uint8Array([
        ...recipient,
        ...extAmount,
        ...relayer,
        ...fee,
        ...merkleTreePubkeyBytes,
        0,
        ...encryptedOutput1,
        ...nonce1,
        ...senderThrowAwayPubkey1,
        ...encryptedOutput2,
        ...nonce2,
        ...senderThrowAwayPubkey2,
        // ...[0],
    ]);
    const hash = ethers_1.ethers.utils.keccak256(Buffer.from(encodedData));
    return {
        extDataHash: ethers_1.BigNumber.from(hash).mod(constants_1.FIELD_SIZE),
        extDataBytes: encodedData,
    };
};
exports.getExtDataHash = getExtDataHash;
