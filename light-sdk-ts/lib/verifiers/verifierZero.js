"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
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
exports.VerifierZero = void 0;
const anchor = __importStar(require("@coral-xyz/anchor"));
const anchor_1 = require("@coral-xyz/anchor");
const web3_js_1 = require("@solana/web3.js");
const index_1 = require("../index");
const spl_token_1 = require("@solana/spl-token");
const index_2 = require("../index");
const verifier_program_zero_1 = require("../idls/verifier_program_zero");
// TODO: Explore alternative architecture in which verifiers inherit/extend or include
// the Transaction class not the other way around like it is right now
class VerifierZero {
    constructor() {
        // Does not work within sdk
        // TODO: bundle files in npm package
        this.wtnsGenPath = "./build-circuits/transactionMasp2_js/transactionMasp2";
        this.zkeyPath = `./build-circuits/transactionMasp2`;
        this.calculateWtns = require("../../build-circuits/transactionMasp2_js/witness_calculator.js");
        this.registeredVerifierPda = index_2.REGISTERED_VERIFIER_PDA;
    }
    parsePublicInputsFromArray(transaction) {
        if (transaction.publicInputsBytes) {
            if (transaction.publicInputsBytes.length == 9) {
                return {
                    root: transaction.publicInputsBytes[0],
                    publicAmount: transaction.publicInputsBytes[1],
                    extDataHash: transaction.publicInputsBytes[2],
                    feeAmount: transaction.publicInputsBytes[3],
                    mintPubkey: transaction.publicInputsBytes[4],
                    nullifiers: [
                        transaction.publicInputsBytes[5],
                        transaction.publicInputsBytes[6],
                    ],
                    leaves: [
                        [
                            transaction.publicInputsBytes[7],
                            transaction.publicInputsBytes[8],
                        ],
                    ],
                };
            }
            else {
                throw `publicInputsBytes.length invalid ${transaction.publicInputsBytes.length} != 9`;
            }
        }
        else {
            throw new Error("public input bytes undefined");
        }
    }
    // TODO: serializeTransaction for relayer
    initVerifierProgram() {
        this.verifierProgram = new anchor_1.Program(verifier_program_zero_1.VerifierProgramZero, index_1.verifierProgramZeroProgramId);
    }
    sendTransaction(transaction) {
        return __awaiter(this, void 0, void 0, function* () {
            this.verifierProgram = new anchor_1.Program(verifier_program_zero_1.VerifierProgramZero, index_1.verifierProgramZeroProgramId);
            console.log("sendTransaction", transaction.recipientFee);
            // await transaction.getPdaAddresses();
            // TODO: move to an init function
            try {
                transaction.recipientBalancePriorTx = (yield (0, spl_token_1.getAccount)(transaction.provider.connection, transaction.recipient, spl_token_1.TOKEN_PROGRAM_ID)).amount;
            }
            catch (e) {
                // covers the case of the recipient being a native sol address not a spl token address
                try {
                    transaction.recipientBalancePriorTx =
                        yield transaction.provider.connection.getBalance(transaction.recipient);
                }
                catch (e) { }
            }
            try {
                transaction.recipientFeeBalancePriorTx =
                    yield transaction.provider.connection.getBalance(transaction.recipientFee);
            }
            catch (error) {
                console.log("transaction.recipientFeeBalancePriorTx fetch failed ", transaction.recipientFee);
            }
            transaction.senderFeeBalancePriorTx =
                yield transaction.provider.connection.getBalance(transaction.senderFee);
            console.log("sendTransaction ");
            transaction.relayerRecipientAccountBalancePriorLastTx =
                yield transaction.provider.connection.getBalance(transaction.relayerRecipient);
            console.log("sendTransaction ");
            // ain derived pda pubkey (Dz5VbR8yVXNg9DsSujFL9mE7U9zTkxBy9NPgH24CEocJ, 254)',
            // 'Program log: Passed-in pda pubkey 7youSP8CcfuvSWxGyJf1JVwaHux6k2Wq15dFPAJUTJvS',
            // 'Program log: Instruction data seed  [32, 221, 13, 181, 197, 157, 122, 91, 137, 50, 220, 253, 86, 14, 185, 235, 248, 65, 247, 142, 135, 232, 230, 228, 140, 194, 44, 128, 82, 67, 8, 147]',
            // "Program log: panicked at 'called `Result::unwrap()` on an `Err` value: InvalidInstructionData', programs/merkle_tree_program/src/verifier_invoked_instructions/insert_nullifier.rs:36:10",
            // 'Program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 consumed 1196752 of 1196752 compute units',
            // 'Program failed to complete: SBF program panicked',
            // console.log("signingAddress:     ", transaction.relayerPubkey)
            // console.log("systemProgram:      ", SystemProgram.programId)
            // console.log("programMerkleTree:  ", transaction.merkleTreeProgram.programId)
            // console.log("rent:               ", DEFAULT_PROGRAMS.rent)
            // console.log("merkleTree:         ", transaction.merkleTreePubkey)
            // console.log("preInsertedLeavesInd", transaction.preInsertedLeavesIndex)
            // console.log("authority:          ", transaction.signerAuthorityPubkey)
            // console.log("tokenProgram:       ", TOKEN_PROGRAM_ID)
            // console.log("sender:             ", transaction.sender)
            // console.log("recipient:          ", transaction.recipient)
            // console.log("senderFee:          ", transaction.senderFee)
            // console.log("senderFee account: ", await transaction.provider.connection.getAccountInfo(transaction.senderFee, "confirmed"));
            // console.log("recipientFee:       ", transaction.recipientFee)
            // console.log("relayerRecipient:   ", transaction.relayerRecipient)
            // console.log("escrow:             ", transaction.escrow)
            // console.log("tokenAuthority:     ", transaction.tokenAuthority)
            // console.log("registeredVerifierPd",transaction.registeredVerifierPda)
            // console.log("transaction.leavesPdaPubkeys ", transaction.leavesPdaPubkeys[0].toBase58());
            // console.log("transaction.signerAuthorityPubkey ", transaction.signerAuthorityPubkey.toBase58());
            const ix = yield transaction.verifier.verifierProgram.methods
                .shieldedTransferInputs(Buffer.from(transaction.proofBytes), Buffer.from(transaction.publicInputs.publicAmount), transaction.publicInputs.nullifiers, transaction.publicInputs.leaves[0], Buffer.from(transaction.publicInputs.feeAmount), new anchor.BN(transaction.rootIndex.toString()), new anchor.BN(transaction.relayerFee.toString()), Buffer.from(transaction.encryptedUtxos.slice(0, 191)) // remaining bytes can be used once tx sizes increase
            )
                .accounts({
                signingAddress: transaction.relayerPubkey,
                systemProgram: web3_js_1.SystemProgram.programId,
                programMerkleTree: transaction.merkleTreeProgram.programId,
                rent: index_1.DEFAULT_PROGRAMS.rent,
                merkleTree: transaction.merkleTreePubkey,
                preInsertedLeavesIndex: transaction.preInsertedLeavesIndex,
                authority: transaction.signerAuthorityPubkey,
                tokenProgram: spl_token_1.TOKEN_PROGRAM_ID,
                sender: transaction.sender,
                recipient: transaction.recipient,
                senderFee: transaction.senderFee,
                recipientFee: transaction.recipientFee,
                relayerRecipient: transaction.relayerRecipient,
                escrow: transaction.escrow,
                tokenAuthority: transaction.tokenAuthority,
                registeredVerifierPda: transaction.verifier.registeredVerifierPda,
            })
                .remainingAccounts([
                {
                    isSigner: false,
                    isWritable: true,
                    pubkey: transaction.nullifierPdaPubkeys[0],
                },
                {
                    isSigner: false,
                    isWritable: true,
                    pubkey: transaction.nullifierPdaPubkeys[1],
                },
                {
                    isSigner: false,
                    isWritable: true,
                    pubkey: transaction.leavesPdaPubkeys[0],
                },
            ])
                .signers([transaction.payer])
                .instruction();
            let recentBlockhash = (yield transaction.provider.connection.getRecentBlockhash("confirmed")).blockhash;
            let txMsg = new web3_js_1.TransactionMessage({
                payerKey: transaction.payer.publicKey,
                instructions: [
                    web3_js_1.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
                    ix,
                ],
                recentBlockhash: recentBlockhash,
            });
            let lookupTableAccount = yield transaction.provider.connection.getAccountInfo(transaction.lookupTable, "confirmed");
            let unpackedLookupTableAccount = web3_js_1.AddressLookupTableAccount.deserialize(lookupTableAccount.data);
            let compiledTx = txMsg.compileToV0Message([
                { state: unpackedLookupTableAccount },
            ]);
            compiledTx.addressTableLookups[0].accountKey = transaction.lookupTable;
            let tx = new web3_js_1.VersionedTransaction(compiledTx);
            let retries = 3;
            let res;
            while (retries > 0) {
                tx.sign([transaction.payer]);
                recentBlockhash = (yield transaction.provider.connection.getRecentBlockhash("confirmed")).blockhash;
                try {
                    let serializedTx = tx.serialize();
                    console.log("serializedTx: ");
                    res = yield (0, web3_js_1.sendAndConfirmRawTransaction)(transaction.provider.connection, serializedTx, index_1.confirmConfig);
                    retries = 0;
                    console.log(res);
                }
                catch (e) {
                    retries--;
                    if (retries == 0 || e.logs != undefined) {
                        console.log(e);
                        return e;
                    }
                }
            }
            return res;
        });
    }
}
exports.VerifierZero = VerifierZero;
