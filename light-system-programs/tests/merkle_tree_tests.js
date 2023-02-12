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
const chai_1 = require("chai");
const token = require("@solana/spl-token");
let circomlibjs = require("circomlibjs");
const light_sdk_1 = require("light-sdk");
const spl_account_compression_1 = require("@solana/spl-account-compression");
var LOOK_UP_TABLE, POSEIDON, KEYPAIR, deposit_utxo1;
console.log = () => { };
describe("Merkle Tree Tests", () => {
    process.env.ANCHOR_WALLET =
        "/Users/" + process.env.USER + "/.config/solana/id.json";
    // Configure the client to use the local cluster.
    var provider = anchor.AnchorProvider.local("http://127.0.0.1:8899", light_sdk_1.confirmConfig);
    anchor.setProvider(provider);
    const merkleTreeProgram = new anchor.Program(light_sdk_1.IDL_MERKLE_TREE_PROGRAM, light_sdk_1.merkleTreeProgramId);
    var INVALID_MERKLE_TREE_AUTHORITY_PDA, INVALID_SIGNER;
    before(() => __awaiter(void 0, void 0, void 0, function* () {
        yield (0, light_sdk_1.createTestAccounts)(provider.connection);
        LOOK_UP_TABLE = yield (0, light_sdk_1.initLookUpTableFromFile)(provider);
        // await setUpMerkleTree(provider);
        var merkleTreeAccountInfoInit = yield provider.connection.getAccountInfo(light_sdk_1.MERKLE_TREE_KEY);
        console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);
        INVALID_SIGNER = new anchor.web3.Account();
        yield provider.connection.confirmTransaction(yield provider.connection.requestAirdrop(INVALID_SIGNER.publicKey, 1000000000000), "confirmed");
        INVALID_MERKLE_TREE_AUTHORITY_PDA = solana.PublicKey.findProgramAddressSync([anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY_INV")], merkleTreeProgram.programId)[0];
    }));
    const test = (fn, obj, error, args) => __awaiter(void 0, void 0, void 0, function* () {
        fn = fn.bind(obj);
        try {
            if (args) {
                (0, chai_1.expect)(yield fn(args)).throw();
                s;
            }
            else {
                (0, chai_1.expect)(yield fn()).throw();
            }
        }
        catch (e) {
            console.log(e);
            chai_1.assert.isTrue(e.logs.includes(error));
        }
    });
    it.skip("Build Merkle Tree from account compression", () => __awaiter(void 0, void 0, void 0, function* () {
        const poseidon = yield circomlibjs.buildPoseidonOpt();
        let merkleTree = yield light_sdk_1.SolMerkleTree.build({
            pubkey: light_sdk_1.MERKLE_TREE_KEY,
            poseidon,
        });
        let newTree = yield merkleTreeProgram.account.merkleTree.fetch(light_sdk_1.MERKLE_TREE_KEY);
        chai_1.assert.equal(merkleTree.merkleTree.root(), new anchor.BN(newTree.roots[newTree.currentRootIndex.toNumber()], 32, "le"));
    }));
    it("Initialize Merkle Tree Test", () => __awaiter(void 0, void 0, void 0, function* () {
        const verifierProgramZero = new anchor.Program(light_sdk_1.IDL_VERIFIER_PROGRAM_ZERO, light_sdk_1.verifierProgramZeroProgramId);
        // const verifierProgramOne = new anchor.Program(VerifierProgramOne, verifierProgramOneProgramId);
        // Security Claims
        // Init authority pda
        // - can only be inited by a hardcoded pubkey
        // Update authority pda
        // - can only be invoked by current authority
        var merkleTreeAccountInfoInit = yield provider.connection.getAccountInfo(light_sdk_1.MERKLE_TREE_KEY);
        console.log("merkleTreeAccountInfoInit ", merkleTreeAccountInfoInit);
        INVALID_SIGNER = new anchor.web3.Account();
        yield provider.connection.confirmTransaction(yield provider.connection.requestAirdrop(INVALID_SIGNER.publicKey, 1000000000000), "confirmed");
        INVALID_MERKLE_TREE_AUTHORITY_PDA = solana.PublicKey.findProgramAddressSync([anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY_INV")], merkleTreeProgram.programId)[0];
        let merkleTreeConfig = new light_sdk_1.MerkleTreeConfig({
            merkleTreePubkey: light_sdk_1.MERKLE_TREE_KEY,
            payer: light_sdk_1.ADMIN_AUTH_KEYPAIR,
            connection: provider.connection,
        });
        yield merkleTreeConfig.getMerkleTreeAuthorityPda();
        let error;
        merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
        try {
            yield merkleTreeConfig.initMerkleTreeAuthority();
        }
        catch (e) {
            error = e;
        }
        yield merkleTreeConfig.getMerkleTreeAuthorityPda();
        console.log(error);
        chai_1.assert.isTrue(error.logs.includes(
        // "Program log: AnchorError caused by account: merkle_tree_authority_pda. Error Code: ConstraintSeeds. Error Number: 2006. Error Message: A seeds constraint was violated."
        "Program log: Instruction: InitializeMerkleTreeAuthority"));
        error = undefined;
        // init merkle tree with invalid signer
        try {
            yield merkleTreeConfig.initMerkleTreeAuthority(INVALID_SIGNER);
            console.log("Registering AUTHORITY success");
        }
        catch (e) {
            error = e;
        }
        console.log(error);
        chai_1.assert.isTrue(error.logs.includes("Program log: Instruction: InitializeMerkleTreeAuthority"));
        error = undefined;
        // initing real mt authority
        yield merkleTreeConfig.initMerkleTreeAuthority();
        yield merkleTreeConfig.initializeNewMerkleTree();
        let newAuthority = new anchor.web3.Account();
        yield provider.connection.confirmTransaction(yield provider.connection.requestAirdrop(newAuthority.publicKey, 1000000000000), "confirmed");
        // update merkle tree with invalid signer
        merkleTreeConfig.payer = INVALID_SIGNER;
        try {
            yield merkleTreeConfig.updateMerkleTreeAuthority(newAuthority.publicKey, true);
            console.log("Registering AUTHORITY success");
        }
        catch (e) {
            error = e;
        }
        chai_1.assert.equal(error.error.errorMessage, "InvalidAuthority");
        error = undefined;
        merkleTreeConfig.payer = light_sdk_1.ADMIN_AUTH_KEYPAIR;
        // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
        merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
        try {
            yield merkleTreeConfig.updateMerkleTreeAuthority(newAuthority.publicKey, true);
            console.log("Registering AUTHORITY success");
        }
        catch (e) {
            error = e;
        }
        yield merkleTreeConfig.getMerkleTreeAuthorityPda();
        chai_1.assert.equal(error.error.errorMessage, "The program expected this account to be already initialized");
        error = undefined;
        yield merkleTreeConfig.updateMerkleTreeAuthority(newAuthority.publicKey);
        merkleTreeConfig.payer = newAuthority;
        yield merkleTreeConfig.updateMerkleTreeAuthority(light_sdk_1.ADMIN_AUTH_KEYPAIR.publicKey);
        merkleTreeConfig.payer = light_sdk_1.ADMIN_AUTH_KEYPAIR;
        // invalid signer
        merkleTreeConfig.payer = INVALID_SIGNER;
        try {
            yield merkleTreeConfig.registerVerifier(verifierProgramZero.programId);
        }
        catch (e) {
            error = e;
        }
        console.log(error);
        chai_1.assert.equal(error.error.errorMessage, "InvalidAuthority");
        error = undefined;
        merkleTreeConfig.payer = light_sdk_1.ADMIN_AUTH_KEYPAIR;
        // invalid pda
        let tmp = merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda;
        merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda =
            INVALID_SIGNER.publicKey;
        try {
            yield merkleTreeConfig.registerVerifier(verifierProgramZero.programId);
        }
        catch (e) {
            error = e;
        }
        console.log(error);
        // assert.equal(error.error.origin, "registered_verifier_pda");
        chai_1.assert.isTrue(error.logs.includes("Program log: Instruction: RegisterVerifier"));
        merkleTreeConfig.registeredVerifierPdas[0].registeredVerifierPda = tmp;
        error = undefined;
        // update merkle tree with invalid signer
        // merkleTreeConfig.payer = INVALID_SIGNER;
        // try {
        //   await merkleTreeConfig.enableNfts(true);
        // } catch (e) {
        //   error = e;
        // }
        // assert.equal(error.error.errorMessage, "InvalidAuthority");
        // error = undefined;
        // merkleTreeConfig.payer = ADMIN_AUTH_KEYPAIR;
        // // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
        // merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
        // try {
        //   await merkleTreeConfig.enableNfts(true);
        // } catch (e) {
        //   error = e;
        // }
        // await merkleTreeConfig.getMerkleTreeAuthorityPda();
        // assert.equal(
        //   error.error.errorMessage,
        //   "The program expected this account to be already initialized"
        // );
        // error = undefined;
        // await merkleTreeConfig.enableNfts(true);
        let merkleTreeAuthority = yield merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda);
        // assert.equal(merkleTreeAuthority.enableNfts, true);
        // await merkleTreeConfig.enableNfts(false);
        // merkleTreeAuthority =
        //   await merkleTreeProgram.account.merkleTreeAuthority.fetch(
        //     merkleTreeConfig.merkleTreeAuthorityPda
        //   );
        // assert.equal(merkleTreeAuthority.enableNfts, false);
        // update lock duration with invalid signer
        merkleTreeConfig.payer = INVALID_SIGNER;
        try {
            yield merkleTreeConfig.updateLockDuration(123);
        }
        catch (e) {
            error = e;
        }
        chai_1.assert.equal(error.error.errorMessage, "InvalidAuthority");
        error = undefined;
        merkleTreeConfig.payer = light_sdk_1.ADMIN_AUTH_KEYPAIR;
        // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
        merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
        try {
            yield merkleTreeConfig.updateLockDuration(123);
        }
        catch (e) {
            error = e;
        }
        yield merkleTreeConfig.getMerkleTreeAuthorityPda();
        chai_1.assert.equal(error.error.errorMessage, "The program expected this account to be already initialized");
        error = undefined;
        yield merkleTreeConfig.updateLockDuration(123);
        yield merkleTreeConfig.updateLockDuration(10);
        // update merkle tree with invalid signer
        merkleTreeConfig.payer = INVALID_SIGNER;
        try {
            yield merkleTreeConfig.enablePermissionlessSplTokens(true);
        }
        catch (e) {
            error = e;
        }
        chai_1.assert.equal(error.error.errorMessage, "InvalidAuthority");
        error = undefined;
        merkleTreeConfig.payer = light_sdk_1.ADMIN_AUTH_KEYPAIR;
        // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
        merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
        try {
            yield merkleTreeConfig.enablePermissionlessSplTokens(true);
        }
        catch (e) {
            error = e;
        }
        yield merkleTreeConfig.getMerkleTreeAuthorityPda();
        chai_1.assert.equal(error.error.errorMessage, "The program expected this account to be already initialized");
        error = undefined;
        yield merkleTreeConfig.enablePermissionlessSplTokens(true);
        merkleTreeAuthority =
            yield merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda);
        chai_1.assert.equal(merkleTreeAuthority.enablePermissionlessSplTokens, true);
        yield merkleTreeConfig.enablePermissionlessSplTokens(false);
        merkleTreeAuthority =
            yield merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda);
        chai_1.assert.equal(merkleTreeAuthority.enablePermissionlessSplTokens, false);
        // update merkle tree with invalid signer
        merkleTreeConfig.payer = INVALID_SIGNER;
        try {
            yield merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));
        }
        catch (e) {
            error = e;
        }
        chai_1.assert.equal(error.error.errorMessage, "InvalidAuthority");
        error = undefined;
        merkleTreeConfig.payer = light_sdk_1.ADMIN_AUTH_KEYPAIR;
        // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
        merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
        try {
            yield merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));
        }
        catch (e) {
            error = e;
        }
        yield merkleTreeConfig.getMerkleTreeAuthorityPda();
        chai_1.assert.equal(error.error.errorMessage, "The program expected this account to be already initialized");
        error = undefined;
        yield merkleTreeConfig.registerPoolType(new Uint8Array(32).fill(0));
        let registeredPoolTypePdaAccount = yield merkleTreeProgram.account.registeredPoolType.fetch(merkleTreeConfig.poolTypes[0].poolPda);
        chai_1.assert.equal(registeredPoolTypePdaAccount.poolType.toString(), new Uint8Array(32).fill(0).toString());
        // update merkle tree with invalid signer
        merkleTreeConfig.payer = INVALID_SIGNER;
        try {
            yield merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));
        }
        catch (e) {
            error = e;
        }
        console.log(error);
        chai_1.assert.equal(error.error.errorMessage, "InvalidAuthority");
        error = undefined;
        merkleTreeConfig.payer = light_sdk_1.ADMIN_AUTH_KEYPAIR;
        // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
        merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
        try {
            yield merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));
        }
        catch (e) {
            error = e;
        }
        yield merkleTreeConfig.getMerkleTreeAuthorityPda();
        console.log("error ", error);
        chai_1.assert.equal(error.error.errorMessage, "The program expected this account to be already initialized");
        error = undefined;
        // valid
        yield merkleTreeConfig.registerSolPool(new Uint8Array(32).fill(0));
        console.log("merkleTreeConfig ", merkleTreeConfig);
        let registeredSolPdaAccount = yield merkleTreeProgram.account.registeredAssetPool.fetch(light_sdk_1.MerkleTreeConfig.getSolPoolPda(light_sdk_1.merkleTreeProgramId).pda);
        chai_1.assert.equal(registeredSolPdaAccount.poolType.toString(), new Uint8Array(32).fill(0).toString());
        chai_1.assert.equal(registeredSolPdaAccount.index, 0);
        chai_1.assert.equal(registeredSolPdaAccount.assetPoolPubkey.toBase58(), light_sdk_1.MerkleTreeConfig.getSolPoolPda(light_sdk_1.merkleTreeProgramId).pda.toBase58());
        let mint = yield (0, light_sdk_1.createMintWrapper)({
            authorityKeypair: light_sdk_1.ADMIN_AUTH_KEYPAIR,
            connection: provider.connection,
        });
        // update merkle tree with invalid signer
        merkleTreeConfig.payer = INVALID_SIGNER;
        try {
            yield merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
        }
        catch (e) {
            error = e;
        }
        chai_1.assert.equal(error.error.errorMessage, "InvalidAuthority");
        error = undefined;
        merkleTreeConfig.payer = light_sdk_1.ADMIN_AUTH_KEYPAIR;
        // update merkle tree with INVALID_MERKLE_TREE_AUTHORITY_PDA
        merkleTreeConfig.merkleTreeAuthorityPda = INVALID_MERKLE_TREE_AUTHORITY_PDA;
        try {
            yield merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
        }
        catch (e) {
            error = e;
        }
        yield merkleTreeConfig.getMerkleTreeAuthorityPda();
        chai_1.assert.equal(error.error.errorMessage, "The program expected this account to be already initialized");
        error = undefined;
        // valid
        yield merkleTreeConfig.registerSplPool(new Uint8Array(32).fill(0), mint);
        console.log(merkleTreeConfig.poolPdas);
        let registeredSplPdaAccount = yield merkleTreeProgram.account.registeredAssetPool.fetch(merkleTreeConfig.poolPdas[0].pda);
        registeredSplPdaAccount =
            yield merkleTreeProgram.account.registeredAssetPool.fetch(merkleTreeConfig.poolPdas[merkleTreeConfig.poolPdas.length - 1].pda);
        console.log(registeredSplPdaAccount);
        chai_1.assert.equal(registeredSplPdaAccount.poolType.toString(), new Uint8Array(32).fill(0).toString());
        chai_1.assert.equal(registeredSplPdaAccount.index.toString(), "1");
        chai_1.assert.equal(registeredSplPdaAccount.assetPoolPubkey.toBase58(), merkleTreeConfig.poolPdas[merkleTreeConfig.poolPdas.length - 1].token.toBase58());
        let merkleTreeAuthority1 = yield merkleTreeProgram.account.merkleTreeAuthority.fetch(merkleTreeConfig.merkleTreeAuthorityPda);
        console.log(merkleTreeAuthority1);
        chai_1.assert.equal(merkleTreeAuthority1.registeredAssetIndex.toString(), "2");
        yield merkleTreeConfig.registerVerifier(verifierProgramZero.programId);
        yield merkleTreeConfig.registerSplPool(light_sdk_1.POOL_TYPE, light_sdk_1.MINT);
        // let nftMint = await createMintWrapper({authorityKeypair: ADMIN_AUTH_KEYPAIR, nft: true, connection: provider.connection})
        // var userTokenAccount = (await newAccountWithTokens({
        //   connection: provider.connection,
        //   MINT: nftMint,
        //   ADMIN_AUTH_KEYPAIR,
        //   userAccount: new anchor.web3.Account(),
        //   amount: 1
        // }))
    }));
    it("deposit ", () => __awaiter(void 0, void 0, void 0, function* () {
        // await createTestAccounts(provider.connection);
        // LOOK_UP_TABLE = await initLookUpTableFromFile(provider);
        // await setUpMerkleTree(provider);
        POSEIDON = yield circomlibjs.buildPoseidonOpt();
        KEYPAIR = new light_sdk_1.Keypair({
            poseidon: POSEIDON,
            seed: light_sdk_1.KEYPAIR_PRIVKEY.toString(),
        });
        var depositAmount = 10000 + (Math.floor(Math.random() * 1000000000) % 1100000000);
        var depositFeeAmount = 10000 + (Math.floor(Math.random() * 1000000000) % 1100000000);
        yield token.approve(provider.connection, light_sdk_1.ADMIN_AUTH_KEYPAIR, light_sdk_1.userTokenAccount, light_sdk_1.Transaction.getSignerAuthorityPda(light_sdk_1.merkleTreeProgramId, new light_sdk_1.VerifierZero().verifierProgram.programId), //delegate
        light_sdk_1.USER_TOKEN_ACCOUNT, // owner
        depositAmount * 10, [light_sdk_1.USER_TOKEN_ACCOUNT]);
        let lightInstance = {
            solMerkleTree: new light_sdk_1.SolMerkleTree({
                pubkey: light_sdk_1.MERKLE_TREE_KEY,
                poseidon: POSEIDON,
            }),
            lookUpTable: LOOK_UP_TABLE,
            provider,
        };
        var transaction = new light_sdk_1.Transaction({
            instance: lightInstance,
            payer: light_sdk_1.ADMIN_AUTH_KEYPAIR,
            shuffleEnabled: false,
        });
        deposit_utxo1 = new light_sdk_1.Utxo({
            poseidon: POSEIDON,
            assets: [light_sdk_1.FEE_ASSET, light_sdk_1.MINT],
            amounts: [new anchor.BN(depositFeeAmount), new anchor.BN(depositAmount)],
            keypair: KEYPAIR,
        });
        let txParams = new light_sdk_1.TransactionParameters({
            outputUtxos: [deposit_utxo1],
            merkleTreePubkey: light_sdk_1.MERKLE_TREE_KEY,
            sender: light_sdk_1.userTokenAccount,
            senderFee: light_sdk_1.ADMIN_AUTH_KEYPAIR.publicKey,
            verifier: new light_sdk_1.VerifierZero(),
        });
        yield transaction.compileAndProve(txParams);
        console.log(transaction.params.accounts);
        // does one successful transaction
        yield transaction.sendAndConfirmTransaction();
    }));
    it("Update Merkle Tree Test", () => __awaiter(void 0, void 0, void 0, function* () {
        // Security Claims
        // CreateUpdateState
        // 1 leaves can only be inserted in the correct index order
        // 2 leaves cannot be inserted twice
        // 3 leaves are queued for a specific tree and can only be inserted in that tree
        // 4 lock is taken and cannot be taken again before expiry
        // 5 Merkle tree is registered
        //
        // Update
        // 6 signer is consistent
        // 7 is locked by update state account
        // 8 merkle tree is consistent
        //
        // Last Tx
        // 9 same leaves as in first tx are marked as inserted
        // 10 is in correct state
        // 11 is locked by update state account
        // 12 merkle tree is consistent
        // 13 signer is consistent
        const signer = light_sdk_1.ADMIN_AUTH_KEYPAIR;
        let mtFetched = yield merkleTreeProgram.account.merkleTree.fetch(light_sdk_1.MERKLE_TREE_KEY);
        let error;
        // fetch uninserted utxos from chain
        let leavesPdas = yield light_sdk_1.SolMerkleTree.getUninsertedLeavesRelayer(light_sdk_1.MERKLE_TREE_KEY);
        let poseidon = yield circomlibjs.buildPoseidonOpt();
        // build tree from chain
        // let merkleTree = await SolMerkleTree.build({pubkey: MERKLE_TREE_KEY, poseidon: POSEIDON})
        let merkleTreeUpdateState = solana.PublicKey.findProgramAddressSync([
            Buffer.from(new Uint8Array(signer.publicKey.toBytes())),
            anchor.utils.bytes.utf8.encode("storage"),
        ], merkleTreeProgram.programId)[0];
        let merkle_tree_pubkey = light_sdk_1.MERKLE_TREE_KEY;
        let connection = provider.connection;
        if (leavesPdas.length > 1) {
            // test leaves with higher starting index than merkle tree next index
            leavesPdas.reverse();
            try {
                const tx1 = yield merkleTreeProgram.methods
                    .initializeMerkleTreeUpdateState()
                    .accounts({
                    authority: signer.publicKey,
                    merkleTreeUpdateState: merkleTreeUpdateState,
                    systemProgram: light_sdk_1.DEFAULT_PROGRAMS.systemProgram,
                    rent: light_sdk_1.DEFAULT_PROGRAMS.rent,
                    merkleTree: merkle_tree_pubkey,
                })
                    .remainingAccounts(leavesPdas)
                    .preInstructions([
                    solana.ComputeBudgetProgram.setComputeUnitLimit({
                        units: 1400000,
                    }),
                ])
                    .signers([signer])
                    .rpc(light_sdk_1.confirmConfig);
                console.log("success 0");
            }
            catch (e) {
                error = e;
            }
            (0, chai_1.assert)(error.error.errorCode.code == "FirstLeavesPdaIncorrectIndex");
            leavesPdas.reverse();
            (0, chai_1.assert)((yield connection.getAccountInfo(merkleTreeUpdateState)) == null);
            console.log("Test property: 1");
            // Test property: 1
            // try with one leavespda of higher index
            try {
                const tx1 = yield merkleTreeProgram.methods
                    .initializeMerkleTreeUpdateState()
                    .accounts({
                    authority: signer.publicKey,
                    merkleTreeUpdateState: merkleTreeUpdateState,
                    systemProgram: web3_js_1.SystemProgram.programId,
                    rent: light_sdk_1.DEFAULT_PROGRAMS.rent,
                    merkleTree: merkle_tree_pubkey,
                })
                    .remainingAccounts(leavesPdas[1])
                    .preInstructions([
                    solana.ComputeBudgetProgram.setComputeUnitLimit({
                        units: 1400000,
                    }),
                ])
                    .signers([signer])
                    .rpc(light_sdk_1.confirmConfig);
                console.log("success 1");
            }
            catch (e) {
                console.log(e);
                error = e;
            }
            (0, chai_1.assert)(error.error.errorCode.code == "FirstLeavesPdaIncorrectIndex");
            (0, chai_1.assert)((yield connection.getAccountInfo(merkleTreeUpdateState)) == null);
        }
        else {
            console.log("pdas.length <=" + 1 + " skipping some tests");
        }
        // Test property: 3
        // try with different Merkle tree than leaves are queued for
        // index might be broken it is wasn't set to mut didn't update
        let merkleTreeConfig = new light_sdk_1.MerkleTreeConfig({
            merkleTreePubkey: light_sdk_1.MERKLE_TREE_KEY,
            payer: light_sdk_1.ADMIN_AUTH_KEYPAIR,
            connection: provider.connection,
        });
        let different_merkle_tree = solana.PublicKey.findProgramAddressSync([
            merkleTreeProgram.programId.toBuffer(),
            new anchor.BN(1).toArray("le", 8),
        ], merkleTreeProgram.programId)[0];
        if ((yield connection.getAccountInfo(different_merkle_tree)) == null) {
            yield merkleTreeConfig.initializeNewMerkleTree(different_merkle_tree);
            console.log("created new merkle tree");
        }
        try {
            const tx1 = yield merkleTreeProgram.methods
                .initializeMerkleTreeUpdateState()
                .accounts({
                authority: signer.publicKey,
                merkleTreeUpdateState: merkleTreeUpdateState,
                systemProgram: web3_js_1.SystemProgram.programId,
                rent: light_sdk_1.DEFAULT_PROGRAMS.rent,
                merkleTree: different_merkle_tree,
            })
                .remainingAccounts(leavesPdas)
                .preInstructions([
                solana.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
            ])
                .signers([signer])
                .rpc(light_sdk_1.confirmConfig);
            console.log("success 3");
        }
        catch (e) {
            console.log(e);
            error = e;
        }
        (0, chai_1.assert)(error.error.errorCode.code == "LeavesOfWrongTree");
        (0, chai_1.assert)((yield connection.getAccountInfo(merkleTreeUpdateState)) == null);
        error = undefined;
        // correct
        try {
            const tx1 = yield merkleTreeProgram.methods
                .initializeMerkleTreeUpdateState()
                .accounts({
                authority: signer.publicKey,
                merkleTreeUpdateState: merkleTreeUpdateState,
                systemProgram: web3_js_1.SystemProgram.programId,
                rent: light_sdk_1.DEFAULT_PROGRAMS.rent,
                merkleTree: merkle_tree_pubkey,
            })
                .remainingAccounts([leavesPdas[0]])
                .preInstructions([
                solana.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
            ])
                .signers([signer])
                .rpc(light_sdk_1.confirmConfig);
        }
        catch (e) {
            error = e;
            console.log(error);
        }
        // should not be an error
        (0, chai_1.assert)(error === undefined);
        console.log("created update state ", merkleTreeUpdateState.toBase58());
        (0, chai_1.assert)((yield connection.getAccountInfo(merkleTreeUpdateState)) != null);
        yield (0, light_sdk_1.checkMerkleTreeUpdateStateCreated)({
            connection: connection,
            merkleTreeUpdateState,
            MerkleTree: merkle_tree_pubkey,
            relayer: signer.publicKey,
            leavesPdas: [leavesPdas[0]],
            current_instruction_index: 1,
            merkleTreeProgram,
        });
        console.log("executeMerkleTreeUpdateTransactions 10");
        yield (0, light_sdk_1.executeMerkleTreeUpdateTransactions)({
            signer,
            merkleTreeProgram,
            merkle_tree_pubkey,
            provider,
            merkleTreeUpdateState,
            numberOfTransactions: 10,
        });
        console.log("checkMerkleTreeUpdateStateCreated 22");
        yield (0, light_sdk_1.checkMerkleTreeUpdateStateCreated)({
            connection: connection,
            merkleTreeUpdateState,
            MerkleTree: merkle_tree_pubkey,
            relayer: signer.publicKey,
            leavesPdas: [leavesPdas[0]],
            current_instruction_index: 22,
            merkleTreeProgram,
        });
        // Test property: 6
        // trying to use merkleTreeUpdateState with different signer
        let maliciousSigner = yield (0, light_sdk_1.newAccountWithLamports)(provider.connection);
        console.log("maliciousSigner: ", maliciousSigner.publicKey.toBase58());
        let maliciousMerkleTreeUpdateState = solana.PublicKey.findProgramAddressSync([
            Buffer.from(new Uint8Array(maliciousSigner.publicKey.toBytes())),
            anchor.utils.bytes.utf8.encode("storage"),
        ], merkleTreeProgram.programId)[0];
        let s = false;
        error = yield (0, light_sdk_1.executeMerkleTreeUpdateTransactions)({
            signer: maliciousSigner,
            merkleTreeProgram,
            merkle_tree_pubkey,
            provider,
            merkleTreeUpdateState,
            numberOfTransactions: 1,
        });
        console.log(error);
        (0, chai_1.assert)(error.logs.includes("Program log: AnchorError caused by account: authority. Error Code: InvalidAuthority. Error Number: 6016. Error Message: InvalidAuthority."));
        // Test property: 4
        // try to take lock
        try {
            const tx1 = yield merkleTreeProgram.methods
                .initializeMerkleTreeUpdateState()
                .accounts({
                authority: maliciousSigner.publicKey,
                merkleTreeUpdateState: maliciousMerkleTreeUpdateState,
                systemProgram: web3_js_1.SystemProgram.programId,
                rent: light_sdk_1.DEFAULT_PROGRAMS.rent,
                merkleTree: merkle_tree_pubkey,
            })
                .remainingAccounts([leavesPdas[0]])
                .signers([maliciousSigner])
                .rpc(light_sdk_1.confirmConfig);
        }
        catch (e) {
            error = e;
            console.log(e);
        }
        (0, chai_1.assert)(error.error.errorCode.code == "ContractStillLocked");
        // Test property: 10
        // try insert root before completing update transaction
        try {
            yield merkleTreeProgram.methods
                .insertRootMerkleTree(new anchor.BN(254))
                .accounts({
                authority: signer.publicKey,
                merkleTreeUpdateState: merkleTreeUpdateState,
                merkleTree: merkle_tree_pubkey,
                logWrapper: spl_account_compression_1.SPL_NOOP_ADDRESS,
            })
                .signers([signer])
                .rpc(light_sdk_1.confirmConfig);
        }
        catch (e) {
            error = e;
        }
        console.log(error);
        (0, chai_1.assert)(error.error.errorCode.code == "MerkleTreeUpdateNotInRootInsert");
        // sending additional tx to finish the merkle tree update
        yield (0, light_sdk_1.executeMerkleTreeUpdateTransactions)({
            signer,
            merkleTreeProgram,
            merkle_tree_pubkey,
            provider,
            merkleTreeUpdateState,
            numberOfTransactions: 50,
        });
        yield (0, light_sdk_1.checkMerkleTreeUpdateStateCreated)({
            connection: connection,
            merkleTreeUpdateState,
            MerkleTree: merkle_tree_pubkey,
            relayer: signer.publicKey,
            leavesPdas: [leavesPdas[0]],
            current_instruction_index: 56,
            merkleTreeProgram,
        });
        // Test property: 11
        // final tx to insert root different UNREGISTERED_MERKLE_TREE
        try {
            console.log("final tx to insert root into different_merkle_tree");
            yield merkleTreeProgram.methods
                .insertRootMerkleTree(new anchor.BN(254))
                .accounts({
                authority: signer.publicKey,
                merkleTreeUpdateState: merkleTreeUpdateState,
                merkleTree: different_merkle_tree,
                logWrapper: spl_account_compression_1.SPL_NOOP_ADDRESS,
            })
                .signers([signer])
                .rpc(light_sdk_1.confirmConfig);
        }
        catch (e) {
            error = e;
        }
        (0, chai_1.assert)(error.error.errorCode.code == "ContractStillLocked");
        // Test property: 13
        // final tx to insert root different signer
        try {
            yield merkleTreeProgram.methods
                .insertRootMerkleTree(new anchor.BN(254))
                .accounts({
                authority: maliciousSigner.publicKey,
                merkleTreeUpdateState: merkleTreeUpdateState,
                merkleTree: merkle_tree_pubkey,
                logWrapper: spl_account_compression_1.SPL_NOOP_ADDRESS,
            })
                .signers([maliciousSigner])
                .rpc(light_sdk_1.confirmConfig);
        }
        catch (e) {
            error = e;
        }
        (0, chai_1.assert)(error.error.errorCode.code == "InvalidAuthority");
        var merkleTreeAccountPrior = yield merkleTreeProgram.account.merkleTree.fetch(merkle_tree_pubkey);
        let merkleTree = yield light_sdk_1.SolMerkleTree.build({
            pubkey: light_sdk_1.MERKLE_TREE_KEY,
            poseidon: POSEIDON,
        });
        // insert correctly
        yield merkleTreeProgram.methods
            .insertRootMerkleTree(new anchor.BN(254))
            .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            merkleTree: merkle_tree_pubkey,
            logWrapper: spl_account_compression_1.SPL_NOOP_ADDRESS,
        })
            .signers([signer])
            .rpc(light_sdk_1.confirmConfig);
        console.log("merkleTreeUpdateState ", merkleTreeUpdateState);
        console.log("merkleTreeAccountPrior ", merkleTreeAccountPrior);
        console.log("leavesPdas[0] ", leavesPdas[0]);
        console.log("merkleTree ", merkleTree);
        console.log("merkle_tree_pubkey ", merkle_tree_pubkey);
        yield (0, light_sdk_1.checkMerkleTreeBatchUpdateSuccess)({
            connection: provider.connection,
            merkleTreeUpdateState: merkleTreeUpdateState,
            merkleTreeAccountPrior,
            numberOfLeaves: 2,
            leavesPdas: [leavesPdas[0]],
            merkleTree: merkleTree,
            merkle_tree_pubkey: merkle_tree_pubkey,
            merkleTreeProgram,
        });
        console.log("Test property: 2");
        // Test property: 2
        // try to reinsert leavesPdas[0]
        try {
            const tx1 = yield merkleTreeProgram.methods
                .initializeMerkleTreeUpdateState()
                .accounts({
                authority: signer.publicKey,
                merkleTreeUpdateState: merkleTreeUpdateState,
                systemProgram: web3_js_1.SystemProgram.programId,
                rent: light_sdk_1.DEFAULT_PROGRAMS.rent,
                merkleTree: merkle_tree_pubkey,
            })
                .remainingAccounts([leavesPdas[0]])
                .preInstructions([
                solana.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
            ])
                .signers([signer])
                .rpc(light_sdk_1.confirmConfig);
        }
        catch (e) {
            error = e;
        }
        (0, chai_1.assert)(error.error.errorCode.code == "LeafAlreadyInserted");
    }));
});
