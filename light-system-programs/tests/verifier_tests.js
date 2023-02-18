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
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const anchor = __importStar(require("@coral-xyz/anchor"));
const web3_js_1 = require("@solana/web3.js");
const solana = require("@solana/web3.js");
const lodash_1 = __importDefault(require("lodash"));
const chai_1 = require("chai");
const token = require("@solana/spl-token");
let circomlibjs = require("circomlibjs");
const light_sdk_1 = require("light-sdk");
const anchor_1 = require("@coral-xyz/anchor");
var LOOK_UP_TABLE, POSEIDON, KEYPAIR, deposit_utxo1;
var transactions = [];
console.log = () => { };
describe("Verifier Zero and One Tests", () => {
    // Configure the client to use the local cluster.
    process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
    const provider = anchor.AnchorProvider.local("http://127.0.0.1:8899", light_sdk_1.confirmConfig);
    process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
    anchor.setProvider(provider);
    const merkleTreeProgram = new anchor.Program(light_sdk_1.IDL_MERKLE_TREE_PROGRAM, light_sdk_1.merkleTreeProgramId);
    var depositAmount, depositFeeAmount;
    const verifiers = [new light_sdk_1.VerifierZero(), new light_sdk_1.VerifierOne()];
    before(() => __awaiter(void 0, void 0, void 0, function* () {
        yield (0, light_sdk_1.createTestAccounts)(provider.connection);
        LOOK_UP_TABLE = yield (0, light_sdk_1.initLookUpTableFromFile)(provider);
        yield (0, light_sdk_1.setUpMerkleTree)(provider);
        POSEIDON = yield circomlibjs.buildPoseidonOpt();
        KEYPAIR = new light_sdk_1.Account({
            poseidon: POSEIDON,
            seed: light_sdk_1.KEYPAIR_PRIVKEY.toString(),
        });
        // overwrite transaction
        depositAmount =
            10000 + (Math.floor(Math.random() * 1000000000) % 1100000000);
        depositFeeAmount =
            10000 + (Math.floor(Math.random() * 1000000000) % 1100000000);
        for (var verifier in verifiers) {
            console.log("verifier ", verifier.toString());
            yield token.approve(provider.connection, light_sdk_1.ADMIN_AUTH_KEYPAIR, light_sdk_1.userTokenAccount, light_sdk_1.Transaction.getSignerAuthorityPda(light_sdk_1.merkleTreeProgramId, verifiers[verifier].verifierProgram.programId), //delegate
            light_sdk_1.USER_TOKEN_ACCOUNT, // owner
            depositAmount * 10, [light_sdk_1.USER_TOKEN_ACCOUNT]);
            let lightProvider = yield light_sdk_1.Provider.native(light_sdk_1.ADMIN_AUTH_KEYPAIR);
            var transaction = new light_sdk_1.Transaction({
                provider: lightProvider,
            });
            deposit_utxo1 = new light_sdk_1.Utxo({
                poseidon: POSEIDON,
                assets: [light_sdk_1.FEE_ASSET, light_sdk_1.MINT],
                amounts: [
                    new anchor.BN(depositFeeAmount),
                    new anchor.BN(depositAmount),
                ],
                account: KEYPAIR,
            });
            let txParams = new light_sdk_1.TransactionParameters({
                outputUtxos: [deposit_utxo1],
                merkleTreePubkey: light_sdk_1.MERKLE_TREE_KEY,
                sender: light_sdk_1.userTokenAccount,
                senderFee: light_sdk_1.ADMIN_AUTH_KEYPAIR.publicKey,
                verifier: verifiers[verifier],
            });
            yield transaction.compileAndProve(txParams);
            // does one successful transaction
            yield transaction.sendAndConfirmTransaction();
            yield (0, light_sdk_1.updateMerkleTreeForTest)(provider);
            // Deposit
            var transaction1 = new light_sdk_1.Transaction({
                provider: lightProvider,
            });
            var deposit_utxo2 = new light_sdk_1.Utxo({
                poseidon: POSEIDON,
                assets: [light_sdk_1.FEE_ASSET, light_sdk_1.MINT],
                amounts: [
                    new anchor.BN(depositFeeAmount),
                    new anchor.BN(depositAmount),
                ],
                account: KEYPAIR,
            });
            let txParams1 = new light_sdk_1.TransactionParameters({
                outputUtxos: [deposit_utxo2],
                merkleTreePubkey: light_sdk_1.MERKLE_TREE_KEY,
                sender: light_sdk_1.userTokenAccount,
                senderFee: light_sdk_1.ADMIN_AUTH_KEYPAIR.publicKey,
                verifier: verifiers[verifier],
            });
            yield transaction1.compileAndProve(txParams1);
            transactions.push(transaction1);
            // Withdrawal
            var tokenRecipient = light_sdk_1.recipientTokenAccount;
            let lightProviderWithdrawal = yield light_sdk_1.Provider.native(light_sdk_1.ADMIN_AUTH_KEYPAIR);
            const relayerRecipient = web3_js_1.Keypair.generate().publicKey;
            yield provider.connection.confirmTransaction(yield provider.connection.requestAirdrop(relayerRecipient, 10000000));
            let relayer = new light_sdk_1.Relayer(light_sdk_1.ADMIN_AUTH_KEYPAIR.publicKey, lightProvider.lookUpTable, relayerRecipient, new anchor_1.BN(100000));
            let tx = new light_sdk_1.Transaction({
                provider: lightProviderWithdrawal,
            });
            let txParams2 = new light_sdk_1.TransactionParameters({
                inputUtxos: [deposit_utxo1],
                merkleTreePubkey: light_sdk_1.MERKLE_TREE_KEY,
                recipient: tokenRecipient,
                recipientFee: light_sdk_1.ADMIN_AUTH_KEYPAIR.publicKey,
                verifier: verifiers[verifier],
                relayer,
            });
            yield tx.compileAndProve(txParams2);
            transactions.push(tx);
        }
    }));
    afterEach(() => __awaiter(void 0, void 0, void 0, function* () {
        // Check that no nullifier was inserted, otherwise the prior test failed
        for (var tx in transactions) {
            yield (0, light_sdk_1.checkNfInserted)(transactions[tx].params.nullifierPdaPubkeys, provider.connection);
        }
    }));
    const sendTestTx = (tx, type, account) => __awaiter(void 0, void 0, void 0, function* () {
        const instructions = yield tx.params.verifier.getInstructions(tx);
        var e;
        for (var ix = 0; ix < instructions.length; ix++) {
            console.log("ix ", ix);
            if (ix != instructions.length - 1) {
                e = yield tx.sendTransaction(instructions[ix]);
                // confirm throws socket hangup error thus waiting a second instead
                yield new Promise((resolve) => setTimeout(resolve, 700));
                // try {
                //     await tx.instance.provider.connection.confirmTransaction(
                //         e
                //     )
                // } catch(error) {console.log(error);
                // }
            }
            else {
                e = yield tx.sendTransaction(instructions[ix]);
            }
        }
        if (type === "ProofVerificationFails") {
            chai_1.assert.isTrue(e.logs.includes("Program log: error ProofVerificationFailed"));
        }
        else if (type === "Account") {
            chai_1.assert.isTrue(e.logs.includes(`Program log: AnchorError caused by account: ${account}. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated.`));
        }
        else if (type === "preInsertedLeavesIndex") {
            chai_1.assert.isTrue(e.logs.includes("Program log: AnchorError caused by account: pre_inserted_leaves_index. Error Code: AccountDiscriminatorMismatch. Error Number: 3002. Error Message: 8 byte discriminator did not match what was expected."));
        }
        else if (type === "Includes") {
            chai_1.assert.isTrue(e.logs.includes(account));
        }
        if (tx.params.verifier.pubkey.toString() ===
            new light_sdk_1.VerifierOne().pubkey.toString()) {
            yield tx.closeVerifierState();
        }
    });
    it("Wrong amount", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            let wrongAmount = new anchor.BN("123213").toArray();
            tmp_tx.publicInputs.publicAmount = Array.from([
                ...new Array(29).fill(0),
                ...wrongAmount,
            ]);
            yield sendTestTx(tmp_tx, "ProofVerificationFails");
        }
    }));
    it("Wrong feeAmount", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            let wrongFeeAmount = new anchor.BN("123213").toArray();
            tmp_tx.publicInputs.feeAmount = Array.from([
                ...new Array(29).fill(0),
                ...wrongFeeAmount,
            ]);
            yield sendTestTx(tmp_tx, "ProofVerificationFails");
        }
    }));
    it("Wrong Mint", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            let relayer = new anchor.web3.Account();
            const newMintKeypair = web3_js_1.Keypair.generate();
            yield (0, light_sdk_1.createMintWrapper)({
                authorityKeypair: light_sdk_1.ADMIN_AUTH_KEYPAIR,
                mintKeypair: newMintKeypair,
                connection: provider.connection,
            });
            tmp_tx.params.accounts.sender = yield (0, light_sdk_1.newAccountWithTokens)({
                connection: provider.connection,
                MINT: newMintKeypair.publicKey,
                ADMIN_AUTH_KEYPAIR: light_sdk_1.ADMIN_AUTH_KEYPAIR,
                userAccount: relayer,
                amount: 0,
            });
            yield sendTestTx(tmp_tx, "ProofVerificationFails");
        }
    }));
    it("Wrong encryptedUtxos", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            tmp_tx.params.encryptedUtxos = new Uint8Array(174).fill(2);
            yield sendTestTx(tmp_tx, "ProofVerificationFails");
        }
    }));
    it("Wrong relayerFee", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            tmp_tx.params.relayer.relayerFee = new anchor.BN("9000");
            yield sendTestTx(tmp_tx, "ProofVerificationFails");
        }
    }));
    it("Wrong nullifier", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            for (var i in tmp_tx.publicInputs.nullifiers) {
                tmp_tx.publicInputs.nullifiers[i] = new Uint8Array(32).fill(2);
                yield sendTestTx(tmp_tx, "ProofVerificationFails");
            }
        }
    }));
    it("Wrong leaves", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            for (var i in tmp_tx.publicInputs.leaves) {
                tmp_tx.publicInputs.leaves[0][i] = new Uint8Array(32).fill(2);
                yield sendTestTx(tmp_tx, "ProofVerificationFails");
            }
        }
    }));
    // doesn't work sig verify error
    it.skip("Wrong signer", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            const wrongSinger = web3_js_1.Keypair.generate();
            yield provider.connection.confirmTransaction(yield provider.connection.requestAirdrop(wrongSinger.publicKey, 1000000000), "confirmed");
            tmp_tx.payer = wrongSinger;
            tmp_tx.relayer.accounts.relayerPubkey = wrongSinger.publicKey;
            yield sendTestTx(tmp_tx, "ProofVerificationFails");
        }
    }));
    it("Wrong recipientFee", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            tmp_tx.params.accounts.recipientFee = web3_js_1.Keypair.generate().publicKey;
            yield sendTestTx(tmp_tx, "ProofVerificationFails");
        }
    }));
    it("Wrong recipient", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            tmp_tx.params.accounts.recipient = web3_js_1.Keypair.generate().publicKey;
            yield sendTestTx(tmp_tx, "ProofVerificationFails");
        }
    }));
    it("Wrong registeredVerifierPda", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            if (tmp_tx.params.accounts.registeredVerifierPda.toBase58() ==
                light_sdk_1.REGISTERED_VERIFIER_ONE_PDA.toBase58()) {
                tmp_tx.params.accounts.registeredVerifierPda = light_sdk_1.REGISTERED_VERIFIER_PDA;
            }
            else {
                tmp_tx.params.accounts.registeredVerifierPda =
                    light_sdk_1.REGISTERED_VERIFIER_ONE_PDA;
            }
            yield sendTestTx(tmp_tx, "Account", "registered_verifier_pda");
        }
    }));
    it("Wrong authority", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            tmp_tx.params.accounts.authority = light_sdk_1.Transaction.getSignerAuthorityPda(light_sdk_1.merkleTreeProgramId, web3_js_1.Keypair.generate().publicKey);
            yield sendTestTx(tmp_tx, "Account", "authority");
        }
    }));
    it("Wrong preInsertedLeavesIndex", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            tmp_tx.params.accounts.preInsertedLeavesIndex = light_sdk_1.REGISTERED_VERIFIER_PDA;
            yield sendTestTx(tmp_tx, "preInsertedLeavesIndex");
        }
    }));
    it("Wrong nullifier accounts", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            for (var i = 0; i < tmp_tx.params.nullifierPdaPubkeys.length; i++) {
                tmp_tx.params.nullifierPdaPubkeys[i] =
                    tmp_tx.params.nullifierPdaPubkeys[(i + 1) % tmp_tx.params.nullifierPdaPubkeys.length];
                yield sendTestTx(tmp_tx, "Includes", "Program log: Passed-in pda pubkey != on-chain derived pda pubkey.");
            }
        }
    }));
    it("Wrong leavesPdaPubkeys accounts", () => __awaiter(void 0, void 0, void 0, function* () {
        for (var tx in transactions) {
            var tmp_tx = lodash_1.default.cloneDeep(transactions[tx]);
            if (tmp_tx.params.leavesPdaPubkeys.length > 1) {
                for (var i = 0; i < tmp_tx.params.leavesPdaPubkeys.length; i++) {
                    tmp_tx.params.leavesPdaPubkeys[i] =
                        tmp_tx.params.leavesPdaPubkeys[(i + 1) % tmp_tx.params.leavesPdaPubkeys.length];
                    yield sendTestTx(tmp_tx, "Includes", "Program log: Passed-in pda pubkey != on-chain derived pda pubkey.");
                }
            }
            else {
                tmp_tx.params.leavesPdaPubkeys[0] = {
                    isSigner: false,
                    isWritable: true,
                    pubkey: web3_js_1.Keypair.generate().publicKey,
                };
                yield sendTestTx(tmp_tx, "Includes", "Program JA5cjkRJ1euVi9xLWsCJVzsRzEkT8vcC4rqw9sVAo5d6 failed: Cross-program invocation with unauthorized signer or writable account");
            }
        }
    }));
});
