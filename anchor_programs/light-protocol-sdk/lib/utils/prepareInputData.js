"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.prepareInputData = void 0;
const ethers_1 = require("ethers");
const constants_1 = require("../constants");
const prepareInputData = (merkelTree, inputUtxos, outputUtxos, inputMerklePathIndices, inputMerklePathElements, externalAmountBigNumber, relayerFee, extDataHash) => {
    return {
        root: merkelTree.root(),
        inputNullifier: inputUtxos.map((x) => x.getNullifier()),
        outputCommitment: outputUtxos.map((x) => x.getCommitment()),
        publicAmount: ethers_1.BigNumber.from(externalAmountBigNumber)
            .sub(ethers_1.BigNumber.from(relayerFee.toString()))
            .add(constants_1.FIELD_SIZE)
            .mod(constants_1.FIELD_SIZE)
            .toString(),
        extDataHash,
        // data for 2 transaction inputUtxos
        inAmount: inputUtxos.map((x) => x.amount),
        inPrivateKey: inputUtxos.map((x) => x.keypair.privkey),
        inBlinding: inputUtxos.map((x) => x.blinding),
        inPathIndices: inputMerklePathIndices,
        inPathElements: inputMerklePathElements,
        // data for 2 transaction outputUtxos
        outAmount: outputUtxos.map((x) => x.amount),
        outBlinding: outputUtxos.map((x) => x.blinding),
        outPubkey: outputUtxos.map((x) => x.keypair.pubkey),
    };
};
exports.prepareInputData = prepareInputData;
