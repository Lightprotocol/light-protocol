"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getExtDataHash = void 0;
const ethers_1 = require("ethers");
const constants_1 = require("../constants");
const getExtDataHash = function (
// inputs are bytes
recipient, extAmount, relayer, fee, merkleTreeIndex, merkleTreePubkeyBytes, encryptedOutput1, encryptedOutput2, nonce1, nonce2, senderThrowAwayPubkey1, senderThrowAwayPubkey2) {
    // console.log("recipient ", Array.prototype.slice.call(recipient))
    // console.log("extAmount ", extAmount)
    // console.log("relayer ", Array.prototype.slice.call(relayer))
    // console.log("fee ", fee)
    // console.log("merkleTreePubkeyBytes ", Array.prototype.slice.call(merkleTreePubkeyBytes))
    // console.log("index merkletreetokenpda ", merkleTreeIndex)
    // console.log("encryptedOutput1 ", encryptedOutput1)
    // console.log("encryptedOutput2 ", encryptedOutput2)
    // console.log("nonce1 ", nonce1)
    // console.log("nonce2 ", nonce2)
    // console.log("senderThrowAwayPubkey1 ", senderThrowAwayPubkey1)
    // console.log("senderThrowAwayPubkey2 ", senderThrowAwayPubkey2)
    // let merkleTreeIndexArr = Uint8Array.from(merkleTreeIndex);
    // console.log("merkleTreeIndexArr", merkleTreeIndexArr)
    let encodedData = new Uint8Array([
        ...recipient,
        ...extAmount,
        ...relayer,
        ...fee,
        ...merkleTreePubkeyBytes,
        merkleTreeIndex,
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
