const anchor = require("@project-serum/anchor")
const constants_1 = require("./../constants");
const enums_1 = require("./enums"); // TODO check if correct bcs js file might be old

const getExternalDataHash_1 = require("./getExternalDataHash");
const parseInputsToBytesArray_1 = require("./parseInputsToBytesArray");
const parseProofToBytesArray_1 = require("./parseProofToBytesArray");

const prove_1 = require("./prove");
const shuffle_1 = require("./shuffle");
const { I64, U64 } = require('n64');
const solana = require('@solana/web3.js');
const nacl = require('tweetnacl');
const newNonce = () => nacl.randomBytes(nacl.box.nonceLength);
const newKeypair = () => nacl.box.keyPair();


export const prepareTransaction = function (inputUtxos = [], outputUtxos = [], merkelTree,merkleTreeIndex,merkleTreePubkeyBytes, externalAmountBigNumber, relayerFee, recipient, relayer, action, encryptionKeypair, inIndices, outIndices, assetPubkeys, mintPubkey, test, feeAmount, recipientFee) {

        console.log(`externalAmountBigNumber -> `, externalAmountBigNumber)

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
        console.log("int64: ", int64);
        let z = new Uint8Array(8);
        int64.writeLE(z, 0);
        console.log("relayerFee: ", relayerFee);
        let feesLE = new Uint8Array(8);
        if (enums_1.Action[action] !== 'deposit') {
            relayerFee.writeLE(feesLE, 0);
        }
        else {
            feesLE.fill(0);
        }

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
        console.log("encryptedOutputs: ", encryptedOutputs.toString())
        console.log("encryptedOutputs.length: ", encryptedOutputs.length);
        console.log("recipientFee: ", recipientFee);
        let encryptedUtxos = new Uint8Array([...encryptedOutputs[0], ...nonces[0], ...senderThrowAwayKeypairs[0].publicKey, ...encryptedOutputs[1], ...nonces[1], ...senderThrowAwayKeypairs[1].publicKey]);
        console.log(relayer);
        const extData = {
            recipient: new solana.PublicKey(recipient).toBytes(),
            recipientFee: recipientFee.toBytes(),
            relayer: new solana.PublicKey(relayer).toBytes(),
            relayer_fee: feesLE,
            merkleTreePubkeyBytes: merkleTreePubkeyBytes
        };
        const { extDataHash, extDataBytes } = (0, getExternalDataHash_1.getExtDataHash)(extData.recipient, extData.recipientFee, extData.relayer, extData.relayer_fee,merkleTreeIndex, encryptedUtxos);

        console.log("feeAmount: ", feeAmount);
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
        console.log("extDataHash: ", input.extDataHash);
        console.log("input.inputNullifier ",input.inputNullifier[0] );
        console.log("input feeAmount: ", input.feeAmount);
        console.log("input publicAmount: ", input.publicAmount);
        console.log("input relayerFee: ", relayerFee);

        return {
                extAmount: extData.extAmount,
                externalAmountBigNumber,
                extDataBytes,
                encryptedUtxos,
                input,
                relayerFee
            };
};

var ffjavascript = require('ffjavascript');
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
export const getProofMasp = async function (input, extAmount, externalAmountBigNumber, extDataBytes, encryptedOutputs, relayerFee) {
        var proofJson;
        var publicInputsJson;
        await (0, prove_1.prove)(input, `./Light_circuits/build/circuits/transactionMasp2`)
            .then((r) => {
            proofJson = r.proofJson;
            publicInputsJson = r.publicInputsJson;
        })

        var publicInputsBytes = JSON.parse(publicInputsJson.toString());
        for (var i in publicInputsBytes) {
            publicInputsBytes[i] = Array.from(leInt2Buff(unstringifyBigInts(publicInputsBytes[i]), 32)).reverse();
        }
        console.log("encryptedOutputs ", encryptedOutputs);
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
        console.log({
                extAmount: extAmount,
                externalAmountBigNumber,
                extDataBytes,
                publicInputs,//
                proofBytes: await (0, parseProofToBytesArray_1.parseProofToBytesArray)(proofJson),
                encryptedOutputs: encryptedOutputs,
                relayerFee
            });
        return {
                extAmount: extAmount,
                externalAmountBigNumber,
                extDataBytes,
                publicInputs,//
                proofBytes: await (0, parseProofToBytesArray_1.parseProofToBytesArray)(proofJson),
                encryptedOutputs: encryptedOutputs,
                relayerFee
            };
};

