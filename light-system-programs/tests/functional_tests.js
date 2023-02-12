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
const anchor = __importStar(require("@coral-xyz/anchor"));
const web3_js_1 = require("@solana/web3.js");
const solana = require("@solana/web3.js");
const token = require("@solana/spl-token");
let circomlibjs = require("circomlibjs");
// TODO: add and use  namespaces in SDK
const light_sdk_1 = require("light-sdk");
const anchor_1 = require("@coral-xyz/anchor");
const account_1 = require("light-sdk/lib/account");
var LOOK_UP_TABLE;
var POSEIDON;
var RELAYER_RECIPIENT;
var KEYPAIR;
var deposit_utxo1;
// TODO: remove deprecated function calls
describe("verifier_program", () => {
    // Configure the client to use the local cluster.
    process.env.ANCHOR_WALLET =
        "/Users/" + process.env.USER + "/.config/solana/id.json";
    const provider = anchor.AnchorProvider.local("http://127.0.0.1:8899", light_sdk_1.confirmConfig);
    anchor.setProvider(provider);
    console.log("merkleTreeProgram: ", light_sdk_1.merkleTreeProgramId.toBase58());
    const merkleTreeProgram = new anchor.Program(light_sdk_1.IDL_MERKLE_TREE_PROGRAM, light_sdk_1.merkleTreeProgramId);
    it("init test setup Merkle tree lookup table etc ", () => __awaiter(void 0, void 0, void 0, function* () {
        yield (0, light_sdk_1.createTestAccounts)(provider.connection);
        LOOK_UP_TABLE = yield (0, light_sdk_1.initLookUpTableFromFile)(provider);
        yield (0, light_sdk_1.setUpMerkleTree)(provider);
        POSEIDON = yield circomlibjs.buildPoseidonOpt();
        KEYPAIR = new account_1.Account({
            poseidon: POSEIDON,
            seed: light_sdk_1.KEYPAIR_PRIVKEY.toString(),
        });
        RELAYER_RECIPIENT = new anchor.web3.Account().publicKey;
    }));
    it.skip("build compressed merkle tree", () => __awaiter(void 0, void 0, void 0, function* () {
        const poseidon = yield circomlibjs.buildPoseidonOpt();
        // await updateMerkleTreeForTest(provider);
        let merkleTree = yield light_sdk_1.SolMerkleTree.build({
            pubkey: light_sdk_1.MERKLE_TREE_KEY,
            poseidon,
        });
        console.log(merkleTree);
    }));
    it("Deposit 10 utxo", () => __awaiter(void 0, void 0, void 0, function* () {
        if (LOOK_UP_TABLE === undefined) {
            throw "undefined LOOK_UP_TABLE";
        }
        let balance = yield provider.connection.getBalance(light_sdk_1.Transaction.getSignerAuthorityPda(merkleTreeProgram.programId, light_sdk_1.verifierProgramOneProgramId), "confirmed");
        if (balance === 0) {
            yield provider.connection.confirmTransaction(yield provider.connection.requestAirdrop(light_sdk_1.Transaction.getSignerAuthorityPda(merkleTreeProgram.programId, light_sdk_1.verifierProgramOneProgramId), 1000000000), "confirmed");
        }
        for (var i = 0; i < 1; i++) {
            console.log("Deposit with 10 utxos ", i);
            let depositAmount = 10000 + Math.floor(Math.random() * 1000000000);
            let depositFeeAmount = 10000 + Math.floor(Math.random() * 1000000000);
            yield token.approve(provider.connection, light_sdk_1.ADMIN_AUTH_KEYPAIR, light_sdk_1.userTokenAccount, light_sdk_1.AUTHORITY_ONE, //delegate
            light_sdk_1.USER_TOKEN_ACCOUNT, // owner
            depositAmount * 2, [light_sdk_1.USER_TOKEN_ACCOUNT]);
            const prov = yield light_sdk_1.Provider.native(light_sdk_1.ADMIN_AUTH_KEYPAIR);
            let tx = new light_sdk_1.Transaction({
                provider: prov,
            });
            let deposit_utxo1 = new light_sdk_1.Utxo({
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
                verifier: new light_sdk_1.VerifierOne(),
            });
            yield tx.compileAndProve(txParams);
            try {
                let res = yield tx.sendAndConfirmTransaction();
                console.log(res);
            }
            catch (e) {
                console.log(e);
            }
            yield tx.checkBalances(KEYPAIR);
            // uncomment below if not running the "deposit" test
            // await updateMerkleTreeForTest(provider);
        }
    }));
    it("Deposit", () => __awaiter(void 0, void 0, void 0, function* () {
        if (LOOK_UP_TABLE === undefined) {
            throw "undefined LOOK_UP_TABLE";
        }
        let depositAmount = 10000 + (Math.floor(Math.random() * 1000000000) % 1100000000);
        let depositFeeAmount = 10000 + (Math.floor(Math.random() * 1000000000) % 1100000000);
        try {
            yield token.approve(provider.connection, light_sdk_1.ADMIN_AUTH_KEYPAIR, light_sdk_1.userTokenAccount, light_sdk_1.AUTHORITY, //delegate
            light_sdk_1.USER_TOKEN_ACCOUNT, // owner
            depositAmount * 2, [light_sdk_1.USER_TOKEN_ACCOUNT]);
            console.log("approved");
        }
        catch (error) {
            console.log(error);
        }
        for (var i = 0; i < 1; i++) {
            console.log("Deposit ", i);
            const prov = yield light_sdk_1.Provider.native(light_sdk_1.ADMIN_AUTH_KEYPAIR);
            let tx = new light_sdk_1.Transaction({
                provider: prov,
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
                verifier: new light_sdk_1.VerifierZero(),
            });
            yield tx.compileAndProve(txParams);
            try {
                let res = yield tx.sendAndConfirmTransaction();
                console.log(res);
            }
            catch (e) {
                console.log(e);
                console.log("AUTHORITY: ", light_sdk_1.AUTHORITY.toBase58());
            }
            yield tx.checkBalances(KEYPAIR);
        }
        yield (0, light_sdk_1.updateMerkleTreeForTest)(provider);
    }));
    it("Withdraw", () => __awaiter(void 0, void 0, void 0, function* () {
        const poseidon = yield circomlibjs.buildPoseidonOpt();
        let merkleTree = yield light_sdk_1.SolMerkleTree.build({
            pubkey: light_sdk_1.MERKLE_TREE_KEY,
            poseidon,
        });
        let leavesPdas = yield light_sdk_1.SolMerkleTree.getInsertedLeaves(light_sdk_1.MERKLE_TREE_KEY);
        let decryptedUtxo1 = yield (0, light_sdk_1.getUnspentUtxo)(leavesPdas, provider, KEYPAIR, POSEIDON, merkleTreeProgram, merkleTree.merkleTree, 0);
        const origin = new anchor.web3.Account();
        var tokenRecipient = light_sdk_1.recipientTokenAccount;
        const prov = yield light_sdk_1.Provider.native(light_sdk_1.ADMIN_AUTH_KEYPAIR);
        let relayer = new light_sdk_1.Relayer(light_sdk_1.ADMIN_AUTH_KEYPAIR.publicKey, prov.lookUpTable, web3_js_1.Keypair.generate().publicKey, new anchor_1.BN(100000));
        let tx = new light_sdk_1.Transaction({
            provider: prov,
            relayer,
            // payer: ADMIN_AUTH_KEYPAIR,
            // shuffleEnabled: false,
        });
        let txParams = new light_sdk_1.TransactionParameters({
            inputUtxos: [decryptedUtxo1],
            merkleTreePubkey: light_sdk_1.MERKLE_TREE_KEY,
            recipient: tokenRecipient,
            recipientFee: origin.publicKey,
            verifier: new light_sdk_1.VerifierZero(),
        });
        yield tx.compileAndProve(txParams);
        // await testTransaction({transaction: SHIELDED_TRANSACTION, deposit: false,provider, signer: ADMIN_AUTH_KEYPAIR, REGISTERED_VERIFIER_ONE_PDA, REGISTERED_VERIFIER_PDA});
        // TODO: add check in client to avoid rent exemption issue
        // add enough funds such that rent exemption is ensured
        yield provider.connection.confirmTransaction(yield provider.connection.requestAirdrop(relayer.accounts.relayerRecipient, 1000000), "confirmed");
        try {
            let res = yield tx.sendAndConfirmTransaction();
            console.log(res);
        }
        catch (e) {
            console.log(e);
            console.log("AUTHORITY: ", light_sdk_1.AUTHORITY.toBase58());
        }
        yield tx.checkBalances();
    }));
    it("Withdraw 10 utxos", () => __awaiter(void 0, void 0, void 0, function* () {
        POSEIDON = yield circomlibjs.buildPoseidonOpt();
        let mtFetched = yield merkleTreeProgram.account.merkleTree.fetch(light_sdk_1.MERKLE_TREE_KEY);
        let merkleTree = yield light_sdk_1.SolMerkleTree.build({
            pubkey: light_sdk_1.MERKLE_TREE_KEY,
            poseidon: POSEIDON,
        });
        let leavesPdas = yield light_sdk_1.SolMerkleTree.getInsertedLeaves(light_sdk_1.MERKLE_TREE_KEY);
        let decryptedUtxo1 = yield (0, light_sdk_1.getUnspentUtxo)(leavesPdas, provider, KEYPAIR, POSEIDON, merkleTreeProgram, merkleTree.merkleTree, 0);
        let inputUtxos = [];
        inputUtxos.push(decryptedUtxo1);
        const relayerRecipient = web3_js_1.Keypair.generate().publicKey;
        const recipientFee = web3_js_1.Keypair.generate().publicKey;
        const prov = yield light_sdk_1.Provider.native(light_sdk_1.ADMIN_AUTH_KEYPAIR);
        yield prov.provider.connection.confirmTransaction(yield prov.provider.connection.requestAirdrop(relayerRecipient, 1000000));
        yield prov.provider.connection.confirmTransaction(yield prov.provider.connection.requestAirdrop(recipientFee, 1000000));
        let relayer = new light_sdk_1.Relayer(light_sdk_1.ADMIN_AUTH_KEYPAIR.publicKey, prov.lookUpTable, relayerRecipient, new anchor_1.BN(100000));
        let tx = new light_sdk_1.Transaction({
            provider: prov,
            relayer,
        });
        let txParams = new light_sdk_1.TransactionParameters({
            inputUtxos,
            outputUtxos: [
                new light_sdk_1.Utxo({
                    poseidon: POSEIDON,
                    assets: inputUtxos[0].assets,
                    amounts: [new anchor_1.BN(0), inputUtxos[0].amounts[1]],
                }),
            ],
            // outputUtxos: [new Utxo({poseidon: POSEIDON, assets: inputUtxos[0].assets, amounts: [inputUtxos[0].amounts[0], new BN(0)]})],
            merkleTreePubkey: light_sdk_1.MERKLE_TREE_KEY,
            recipient: light_sdk_1.recipientTokenAccount,
            recipientFee,
            verifier: new light_sdk_1.VerifierOne(),
        });
        yield tx.compileAndProve(txParams);
        try {
            let res = yield tx.sendAndConfirmTransaction();
            console.log(res);
        }
        catch (e) {
            console.log(e);
        }
        yield tx.checkBalances();
    }));
});
