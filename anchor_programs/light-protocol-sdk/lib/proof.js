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

var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
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


const prepareTransaction = function (inputUtxos = [], outputUtxos = [], merkelTree,merkleTreeIndex,merkleTreePubkeyBytes, externalAmountBigNumber, relayerFee, recipient, relayer, action, encryptionKeypair, inIndices, outIndices, assetPubkeys, mintPubkey, test, feeAmount, recipientFee) {
    return __awaiter(this, void 0, void 0, function* () {
        /// mixes the input utxos
        /// mixes the output utxos
        // inputUtxos = (0, shuffle_1.shuffle)(inputUtxos);
        // outputUtxos = (0, shuffle_1.shuffle)(outputUtxos);
        // console.log(`input utxos -> `, inputUtxos)
        // console.log(`outputUtxos -> `, outputUtxos)
        // console.log(`merkelTree -> `, merkelTree)
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

        const nonces = [newNonce(), newNonce()];
        // const senderThrowAwayKeypairs = [
        //     newKeypair(),
        //     newKeypair()
        // ];
        // console.log(outputUtxos)
        /// Encrypt outputUtxos to bytes
        // removed throwaway keypairs since we already have message integrity with integrity_hashes
        // TODO: should be a hardcoded keypair in production not the one of the sender
        let encryptedOutputs = [ ];
        outputUtxos.map((utxo, index) => encryptedOutputs.push(utxo.encrypt(nonces[index], encryptionKeypair, encryptionKeypair)));

        // console.log("removed senderThrowAwayKeypairs TODO: always use fixed keypair or switch to salsa20 without poly153");
        let encryptedUtxos = new Uint8Array([...encryptedOutputs[0], ...nonces[0], ...encryptedOutputs[1], ...nonces[1], ...new Array(256 - 174).fill(0)]);

        const extData = {
            recipient: new solana.PublicKey(recipient).toBytes(),
            recipientFee: recipientFee.toBytes(),
            relayer: new solana.PublicKey(relayer).toBytes(),
            relayer_fee: feesLE,
            merkleTreePubkeyBytes: merkleTreePubkeyBytes
        };
        const { extDataHash, extDataBytes } = (0, getExternalDataHash_1.getExtDataHash)(extData.recipient, extData.recipientFee, extData.relayer, extData.relayer_fee,merkleTreeIndex, encryptedUtxos);

        let input = {
            root: merkelTree.root(),
            inputNullifier: inputUtxos.map((x) => x.getNullifier()),
            outputCommitment: outputUtxos.map((x) => x.getCommitment()),
            // TODO: move public and fee amounts into tx preparation
            publicAmount: new anchor.BN(externalAmountBigNumber)
                .add(constants_1.FIELD_SIZE)
                .mod(constants_1.FIELD_SIZE)
                .toString(),
            extDataHash: extDataHash.toString(),
            feeAmount: new anchor.BN(feeAmount)
                .add(constants_1.FIELD_SIZE)
                .mod(constants_1.FIELD_SIZE)
                .toString(),
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
        // console.log("extDataHash: ", input.extDataHash);
        // console.log("input.inputNullifier ",input.inputNullifier[0] );
        // console.log("input feeAmount: ", input.feeAmount);
        // console.log("input publicAmount: ", input.publicAmount);
        // console.log("input relayerFee: ", relayerFee);
        //
        // console.log("inIndices ", JSON.stringify(inIndices, null, 4));
        // console.log("outIndices ", JSON.stringify(outIndices, null, 4));

        return {
                extAmount: extData.extAmount,
                externalAmountBigNumber,
                extDataBytes,
                encryptedUtxos,
                input,
                relayerFee
            };
    });
};
exports.prepareTransaction = prepareTransaction;


const getProofMasp = function (input, extAmount, externalAmountBigNumber, extDataBytes, encryptedOutputs, relayerFee) {
    return __awaiter(this, void 0, void 0, function* () {
        console.log(input);
        var proofJson;
        var publicInputsJson;
        yield (0, prove_1.prove)(input)
            .then((r) => {
            proofJson = r.proofJson;
            publicInputsJson = r.publicInputsJson;
        })

        var publicInputsBytes = JSON.parse(publicInputsJson.toString());
        for (var i in publicInputsBytes) {
            publicInputsBytes[i] = Array.from(leInt2Buff(unstringifyBigInts(publicInputsBytes[i]), 32)).reverse();
        }
        console.log("publicInputsBytes ", publicInputsBytes);
        let publicInputs

        if (publicInputsBytes.length == 9) {
            publicInputs = {
             root:         publicInputsBytes[0],
             publicAmount: publicInputsBytes[1],
             extDataHash:  publicInputsBytes[2],
             feeAmount:    publicInputsBytes[3],
             mintPubkey:   publicInputsBytes[4],
             nullifiers:   [publicInputsBytes[5], publicInputsBytes[6]],
             leaves:     [publicInputsBytes[7], publicInputsBytes[8]]
           };
        } else {
          publicInputs = {
           root:         publicInputsBytes[0],
           publicAmount: publicInputsBytes[1],
           extDataHash:  publicInputsBytes[2],
           feeAmount:    publicInputsBytes[3],
           mintPubkey:   publicInputsBytes[4],
           nullifiers:   Array.from(publicInputsBytes.slice(5,15)),
           leaves:     [publicInputsBytes[15], publicInputsBytes[16]]
         };
        }

        return {
                extAmount: extAmount,
                externalAmountBigNumber,
                extDataBytes,
                publicInputs,//
                proofBytes: yield (0, parseProofToBytesArray_1.parseProofToBytesArray)(proofJson),
                encryptedOutputs: encryptedOutputs,
                relayerFee
            };
    });
};

exports.getProofMasp = getProofMasp;
