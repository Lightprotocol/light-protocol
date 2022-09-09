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
exports.prepareTransaction = void 0;
exports.getProofMasp = void 0;
const anchor = require("@project-serum/anchor")

const constants_1 = require("./constants");
const enums_1 = require("./enums");
const getExternalDataHash_1 = require("./utils/getExternalDataHash");
const parseInputsToBytesArray_1 = require("./utils/parseInputsToBytesArray");
const parseProofToBytesArray_1 = require("./utils/parseProofToBytesArray");
const prove_1 = require("./utils/prove");
const shuffle_1 = require("./utils/shuffle");
const { I64, U64 } = require('n64');
const solana = require('@solana/web3.js');
const nacl = require('tweetnacl');
const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
const newKeypair = () => nacl.box.keyPair();


const prepareTransaction = function (inputUtxos = [], outputUtxos = [], merkelTree,merkleTreeIndex,merkleTreePubkeyBytes, externalAmountBigNumber, relayerFee, recipient, relayer, action, encryptionKeypair, inIndices, outIndices, assetPubkeys, mintPubkey, test) {
    return __awaiter(this, void 0, void 0, function* () {
        /// mixes the input utxos
        /// mixes the output utxos
        // inputUtxos = (0, shuffle_1.shuffle)(inputUtxos);
        // outputUtxos = (0, shuffle_1.shuffle)(outputUtxos);
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
            if (test) {
              inputMerklePathIndices.push(0);
              inputMerklePathElements.push(new Array(merkelTree.levels).fill(0));

            }
            else if (inputUtxo.amounts[0] > 0 || inputUtxo.amounts[1] > 0|| inputUtxo.amounts[2] > 0)  {
                inputUtxo.index = merkelTree.indexOf(inputUtxo.getCommitment());
                if (inputUtxo.index || inputUtxo.index == 0) {
                    if (inputUtxo.index < 0) {
                        throw new Error(`Input commitment ${inputUtxo.getCommitment()} was not found`);
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
        if (externalAmountBigNumber < 0) {
            // is withdrawal
            int64 = I64(-1 * externalAmountBigNumber.toNumber());
        }
        else {
            int64 = I64(externalAmountBigNumber.toNumber());
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
        // Encrypting outputUtxos onl11706303591708973095148360617010962626840972903207922381194541855451929915886y
        // Why is this empty
        const nonces = [newNonce(), newNonce()];
        const senderThrowAwayKeypairs = [
            newKeypair(),
            newKeypair()
        ];
        // console.log(outputUtxos)
        /// Encrypt outputUtxos to bytes
        let encryptedOutputs = [ ];
        outputUtxos.map((utxo, index) => encryptedOutputs.push(utxo.encrypt(nonces[index], encryptionKeypair, senderThrowAwayKeypairs[index])));
        // test decrypt: same?
        console.log("encryptedOutputs: ", encryptedOutputs)
        console.log("encryptedOutputs.length: ", encryptedOutputs.length);

        const extData = {
            recipient: new solana.PublicKey(recipient).toBytes(),
            extAmount: z,
            relayer: new solana.PublicKey(relayer).toBytes(),
            fee: feesLE,
            merkleTreePubkeyBytes: merkleTreePubkeyBytes,
            encryptedOutput1: encryptedOutputs[0],
            encryptedOutput2: encryptedOutputs[1],
            nonce1: nonces[0],
            nonce2: nonces[1],
            senderThrowAwayPubkey1: senderThrowAwayKeypairs[0].publicKey,
            senderThrowAwayPubkey2: senderThrowAwayKeypairs[1].publicKey,
        };
        const { extDataHash, extDataBytes } = (0, getExternalDataHash_1.getExtDataHash)(extData.recipient, extData.extAmount, extData.relayer, extData.fee,merkleTreeIndex, extData.merkleTreePubkeyBytes, extData.encryptedOutput1, extData.encryptedOutput2, extData.nonce1, extData.nonce2, extData.senderThrowAwayPubkey1, extData.senderThrowAwayPubkey2);
        let feeAmount = new anchor.BN(relayerFee.toString())
        if (externalAmountBigNumber < 0) {
          feeAmount = new anchor.BN(0).sub(new anchor.BN(relayerFee.toString()))
          .add(constants_1.FIELD_SIZE)
          .mod(constants_1.FIELD_SIZE)
          .toString()
        }

        let input = {
            root: merkelTree.root(),
            inputNullifier: inputUtxos.map((x) => x.getNullifier()),
            outputCommitment: outputUtxos.map((x) => x.getCommitment()),
            publicAmount: new anchor.BN(externalAmountBigNumber)
                .add(constants_1.FIELD_SIZE)
                .mod(constants_1.FIELD_SIZE)
                .toString(),
            extDataHash,
            feeAmount,
            mintPubkey,
            // data for 2 transaction inputUtxos
            inAmount: inputUtxos.map((x) => x.amounts),
            inPrivateKey: inputUtxos.map((x) => x.keypair.privkey),
            inBlinding: inputUtxos.map((x) => x.blinding),
            inPathIndices: inputMerklePathIndices,
            inPathElements: inputMerklePathElements,
            assetPubkeys,
            // data for 2 transaction outputUtxos
            outAmount: outputUtxos.map((x) => x.amounts),
            outBlinding: outputUtxos.map((x) => x.blinding),
            outPubkey: outputUtxos.map((x) => x.keypair.pubkey),
            inIndices,
            outIndices,
            inInstructionType: inputUtxos.map((x) => x.instructionType),
            outInstructionType: outputUtxos.map((x) => x.instructionType)
        };

        return {
                extAmount: extData.extAmount,
                externalAmountBigNumber,
                extDataBytes,
                encryptedOutputs,
                input
            };
    });
};
exports.prepareTransaction = prepareTransaction;

var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
const getProofMasp = function (input, extAmount, externalAmountBigNumber, extDataBytes, encryptedOutputs) {
    return __awaiter(this, void 0, void 0, function* () {
        var proofJson;
        var publicInputsJson;
        yield (0, prove_1.prove)(input, `./Light_circuits/build/circuits/transactionMasp2`)
            .then((r) => {
            proofJson = r.proofJson;
            publicInputsJson = r.publicInputsJson;
        })

        var publicInputsBytes = JSON.parse(publicInputsJson.toString());
        for (var i in publicInputsBytes) {
            publicInputsBytes[i] = Array.from(leInt2Buff(unstringifyBigInts(publicInputsBytes[i]), 32)).reverse();
        }

        let publicInputs = {
            root:         publicInputsBytes[0],
            publicAmount: publicInputsBytes[1],
            extDataHash:  publicInputsBytes[2],
            feeAmount:    publicInputsBytes[3],
            mintPubkey:   publicInputsBytes[4],
            nullifier0:   publicInputsBytes[5],
            nullifier1:   publicInputsBytes[6],
            leafLeft:     publicInputsBytes[7],
            leafRight:    publicInputsBytes[8]
        };

        return {
            data: {
                extAmount: extAmount,
                externalAmountBigNumber,
                extDataBytes,
                publicInputs,//
                proofBytes: yield (0, parseProofToBytesArray_1.parseProofToBytesArray)(proofJson),
                encryptedOutputs: encryptedOutputs,
            },
        };
    });
};

exports.getProofMasp = getProofMasp;
