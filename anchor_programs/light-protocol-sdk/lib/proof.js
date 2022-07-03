"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.getProof = void 0;
const ethers_1 = require("ethers");
const constants_1 = require("./constants");
const enums_1 = require("./enums");
const getExternalDataHash_1 = require("./utils/getExternalDataHash");
const parseInputsToBytesArray_1 = require("./utils/parseInputsToBytesArray");
const parseProofToBytesArray_1 = require("./utils/parseProofToBytesArray");
const prove_1 = require("./utils/prove");
const shuffle_1 = require("./utils/shuffle");
const timeoutPromise_1 = require("./utils/timeoutPromise");
const toFixedHex_1 = require("./utils/toFixedHex");
const { I64, U64 } = require('n64');
const solana = require('@solana/web3.js');
// TODO could be in helper function
const nacl = require('tweetnacl');
const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
const newKeypair = () => nacl.box.keyPair();
//
const getProof = function (inputUtxos = [], outputUtxos = [], merkelTree, externalAmountBigNumber, relayerFee, recipient, relayer, action, encryptionKeypair) {
    return __awaiter(this, void 0, void 0, function* () {
        /// mixes the input utxos
        /// mixes the output utxos
        inputUtxos = (0, shuffle_1.shuffle)(inputUtxos);
        outputUtxos = (0, shuffle_1.shuffle)(outputUtxos);
        // console.log(`input utxos -> `, inputUtxos)
        // console.log(`outputUtxos -> `, outputUtxos)
        // console.log(`merkelTree -> `, merkelTree)
        // console.log(`externalAmountBigNumber -> `, externalAmountBigNumber)
        // console.log(`relayerFee -> `, relayerFee)
        // console.log(`recipient -> `, recipient)
        // console.log(`Action[action] -> `, Action[action])
        // console.log(`encryptionKeypair -> `, encryptionKeypair)
        let inputMerklePathIndices = [];
        let inputMerklePathElements = [];
        /// if the input utxo has an amount bigger than 0 and it has an valid index add it to the indices of the merkel tree
        /// also push the path to the leaf
        /// else push a 0 to the indices
        /// and fill the path to the leaf with 0s
        for (const inputUtxo of inputUtxos) {
            if (inputUtxo.amount > 0) {
                inputUtxo.index = merkelTree.indexOf((0, toFixedHex_1.toFixedHex)(inputUtxo.getCommitment()));
                if (inputUtxo.index) {
                    if (inputUtxo.index < 0) {
                        throw new Error(`Input commitment ${(0, toFixedHex_1.toFixedHex)(inputUtxo.getCommitment())} was not found`);
                    }
                    inputMerklePathIndices.push(inputUtxo.index);
                    inputMerklePathElements.push(merkelTree.path(inputUtxo.index).pathElements);
                }
            }
            else {
                inputMerklePathIndices.push(0);
                inputMerklePathElements.push(new Array(merkelTree.levels).fill(0));
            }
        }
        // does something with the fees
        let int64;
        if (externalAmountBigNumber._hex.indexOf('-') > -1) {
            // is withdrawal
            int64 = I64(-1 * Number(externalAmountBigNumber._hex.slice(1)));
        }
        else {
            int64 = I64(Number(externalAmountBigNumber._hex));
        }
        let z = new Uint8Array(8);
        int64.writeLE(z, 0);
        let feesLE = new Uint8Array(8);
        if (enums_1.Action[action] !== 'deposit') {
            relayerFee.writeLE(feesLE, 0);
        }
        else {
            feesLE.fill(0);
        }
        // Encrypting outputUtxos only
        // Why is this empty
        const nonces = [newNonce(), newNonce()];
        const senderThrowAwayKeypairs = [
            newKeypair(),
            newKeypair(),
        ];
        /// Encrypt outputUtxos to bytes
        let encryptedOutputs = [];
        outputUtxos.map((utxo, index) => encryptedOutputs.push(utxo.encrypt(nonces[index], encryptionKeypair, senderThrowAwayKeypairs[index])));
        // test decrypt: same?
        const extData = {
            recipient: new solana.PublicKey(recipient).toBytes(),
            extAmount: z,
            relayer: new solana.PublicKey(relayer).toBytes(),
            fee: feesLE,
            merkleTreePubkeyBytes: new solana.PublicKey(constants_1.REACT_APP_MERKLE_TREE_PDA_PUBKEY).toBytes(),
            encryptedOutput1: encryptedOutputs[0],
            encryptedOutput2: encryptedOutputs[1],
            nonce1: nonces[0],
            nonce2: nonces[1],
            senderThrowAwayPubkey1: senderThrowAwayKeypairs[0].publicKey,
            senderThrowAwayPubkey2: senderThrowAwayKeypairs[1].publicKey,
        };
        const { extDataHash, extDataBytes } = (0, getExternalDataHash_1.getExtDataHash)(extData.recipient, extData.extAmount, extData.relayer, extData.fee, extData.merkleTreePubkeyBytes, extData.encryptedOutput1, extData.encryptedOutput2, extData.nonce1, extData.nonce2, extData.senderThrowAwayPubkey1, extData.senderThrowAwayPubkey2);
        let input = {
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
        var proofJson;
        var publicInputsJson;
        yield (0, timeoutPromise_1.timeoutPromise)(40, (0, prove_1.prove)(input, `./artifacts/circuits/transaction${inputUtxos.length}`))
            .then((r) => {
            proofJson = r.proofJson;
            publicInputsJson = r.publicInputsJson;
        })
            .catch((e) => {
            console.log(e);
            throw new Error(`Your proof generation took too long. Please refresh the page and try again.`);
        });
        return {
            data: {
                extAmount: extData.extAmount,
                externalAmountBigNumber,
                extDataBytes,
                publicInputsBytes: yield (0, parseInputsToBytesArray_1.parseInputsToBytesArray)(publicInputsJson),
                proofBytes: yield (0, parseProofToBytesArray_1.parseProofToBytesArray)(proofJson),
                encryptedOutputs: encryptedOutputs, // may need these somewhere
            },
        };
    });
};
exports.getProof = getProof;
