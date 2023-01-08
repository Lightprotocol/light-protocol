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
exports.MerkleTreeConfig = void 0;
const anchor = __importStar(require("@project-serum/anchor"));
const chai_1 = require("chai");
const token = require('@solana/spl-token');
const web3_js_1 = require("@solana/web3.js");
const constants_1 = require("../constants");
class MerkleTreeConfig {
    constructor({ merkleTreePubkey, payer, connection }) {
        this.merkleTreePubkey = merkleTreePubkey;
        this.payer = payer;
        this.merkleTreeProgram = constants_1.merkleTreeProgram;
        // TODO: reorg pool pdas, have one object per pool type and then an array with registered pools of this type
        this.poolPdas = [];
        this.poolTypes = [];
        this.registeredVerifierPdas = [];
        this.connection = connection;
    }
    getPreInsertedLeavesIndex() {
        return __awaiter(this, void 0, void 0, function* () {
            this.preInsertedLeavesIndex = (yield web3_js_1.PublicKey.findProgramAddress([this.merkleTreePubkey.toBuffer()], this.merkleTreeProgram.programId))[0];
            return this.preInsertedLeavesIndex;
        });
    }
    initializeNewMerkleTree(merkleTreePubkey) {
        return __awaiter(this, void 0, void 0, function* () {
            if (merkleTreePubkey) {
                this.merkleTreePubkey = merkleTreePubkey;
            }
            yield this.getPreInsertedLeavesIndex();
            yield this.getMerkleTreeAuthorityPda();
            const tx = yield this.merkleTreeProgram.methods.initializeNewMerkleTree(new anchor.BN("50")).accounts({
                authority: this.payer.publicKey,
                merkleTree: this.merkleTreePubkey,
                preInsertedLeavesIndex: this.preInsertedLeavesIndex,
                systemProgram: constants_1.DEFAULT_PROGRAMS.systemProgram,
                rent: constants_1.DEFAULT_PROGRAMS.rent,
                merkleTreeAuthorityPda: this.merkleTreeAuthorityPda
            })
                .signers([this.payer])
                .rpc(constants_1.confirmConfig);
            yield this.checkMerkleTreeIsInitialized();
            yield this.checkPreInsertedLeavesIndexIsInitialized();
            return tx;
        });
    }
    checkMerkleTreeIsInitialized() {
        return __awaiter(this, void 0, void 0, function* () {
            var merkleTreeAccountInfo = yield this.merkleTreeProgram.account.merkleTree.fetch(this.merkleTreePubkey);
            (0, chai_1.assert)(merkleTreeAccountInfo != null, "merkleTreeAccountInfo not initialized");
            // zero values
            // index == 0
            // roots are empty save for 0
            // lock duration is correct
            (0, chai_1.assert)(merkleTreeAccountInfo.lockDuration.toString() == "50");
        });
    }
    checkPreInsertedLeavesIndexIsInitialized() {
        return __awaiter(this, void 0, void 0, function* () {
            var preInsertedLeavesIndexAccountInfo = yield this.merkleTreeProgram.account.preInsertedLeavesIndex.fetch(this.preInsertedLeavesIndex);
            (0, chai_1.assert)(preInsertedLeavesIndexAccountInfo != null, "preInsertedLeavesIndexAccountInfo not initialized");
            (0, chai_1.assert)(preInsertedLeavesIndexAccountInfo.nextIndex.toString() == "0");
        });
    }
    printMerkleTree() {
        return __awaiter(this, void 0, void 0, function* () {
            var merkleTreeAccountInfo = yield this.merkleTreeProgram.account.merkleTree.fetch(this.merkleTreePubkey);
            console.log("merkleTreeAccountInfo ", merkleTreeAccountInfo);
        });
    }
    getMerkleTreeAuthorityPda() {
        return __awaiter(this, void 0, void 0, function* () {
            this.merkleTreeAuthorityPda = (yield web3_js_1.PublicKey.findProgramAddress([anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY")], this.merkleTreeProgram.programId))[0];
            return this.merkleTreeAuthorityPda;
        });
    }
    initMerkleTreeAuthority(authority) {
        return __awaiter(this, void 0, void 0, function* () {
            if (authority == undefined) {
                authority = this.payer;
            }
            if (this.merkleTreeAuthorityPda == undefined) {
                yield this.getMerkleTreeAuthorityPda();
            }
            const tx = yield this.merkleTreeProgram.methods.initializeMerkleTreeAuthority().accounts(Object.assign({ authority: authority.publicKey, merkleTreeAuthorityPda: this.merkleTreeAuthorityPda }, constants_1.DEFAULT_PROGRAMS))
                .signers([authority])
                .rpc(constants_1.confirmConfig);
            // await sendAndConfirmTransaction(this.connection, new Transaction([authority]).add(tx), [authority], confirmConfig);
            // rpc(confirmConfig);
            (0, chai_1.assert)(this.connection.getAccountInfo(this.merkleTreeAuthorityPda, "confirmed") != null, "init authority failed");
            let merkleTreeAuthority = yield this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda);
            (0, chai_1.assert)(merkleTreeAuthority.enablePermissionlessSplTokens == false);
            (0, chai_1.assert)(merkleTreeAuthority.enableNfts == false);
            (0, chai_1.assert)(merkleTreeAuthority.pubkey.toBase58() == authority.publicKey.toBase58());
            (0, chai_1.assert)(merkleTreeAuthority.registeredAssetIndex.toString() == "0");
            return tx;
        });
    }
    updateMerkleTreeAuthority(newAuthority, test = false) {
        return __awaiter(this, void 0, void 0, function* () {
            if (!this.merkleTreeAuthorityPda) {
                yield this.getMerkleTreeAuthorityPda();
            }
            let merkleTreeAuthorityPrior;
            if (test != true) {
                merkleTreeAuthorityPrior = yield this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda);
                if (merkleTreeAuthorityPrior == null) {
                    throw `Merkle tree authority ${this.merkleTreeAuthorityPda.toBase58()} not initialized`;
                }
            }
            const tx = yield this.merkleTreeProgram.methods.updateMerkleTreeAuthority().accounts(Object.assign({ authority: this.payer.publicKey, newAuthority, merkleTreeAuthorityPda: this.merkleTreeAuthorityPda }, constants_1.DEFAULT_PROGRAMS))
                .signers([this.payer])
                .rpc(constants_1.confirmConfig);
            if (test != true) {
                let merkleTreeAuthority = yield this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda);
                (0, chai_1.assert)(merkleTreeAuthority.enablePermissionlessSplTokens == merkleTreeAuthorityPrior.enablePermissionlessSplTokens);
                (0, chai_1.assert)(merkleTreeAuthority.enableNfts == merkleTreeAuthorityPrior.enableNfts);
                (0, chai_1.assert)(merkleTreeAuthority.pubkey.toBase58() == newAuthority.toBase58());
            }
            return tx;
        });
    }
    enableNfts(configValue) {
        return __awaiter(this, void 0, void 0, function* () {
            if (this.merkleTreeAuthorityPda == undefined) {
                yield this.getMerkleTreeAuthorityPda();
            }
            const tx = yield this.merkleTreeProgram.methods.enableNfts(configValue).accounts(Object.assign({ authority: this.payer.publicKey, merkleTreeAuthorityPda: this.merkleTreeAuthorityPda }, constants_1.DEFAULT_PROGRAMS))
                .signers([this.payer])
                .rpc(constants_1.confirmConfig);
            let merkleTreeAuthority = yield this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda);
            (0, chai_1.assert)(merkleTreeAuthority.enableNfts == configValue);
            return tx;
        });
    }
    enablePermissionlessSplTokens(configValue) {
        return __awaiter(this, void 0, void 0, function* () {
            if (this.merkleTreeAuthorityPda == undefined) {
                yield this.getMerkleTreeAuthorityPda();
            }
            const tx = yield this.merkleTreeProgram.methods.enablePermissionlessSplTokens(configValue).accounts(Object.assign({ authority: this.payer.publicKey, merkleTreeAuthorityPda: this.merkleTreeAuthorityPda }, constants_1.DEFAULT_PROGRAMS))
                .signers([this.payer])
                .rpc(constants_1.confirmConfig);
            let merkleTreeAuthority = yield this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda);
            (0, chai_1.assert)(merkleTreeAuthority.enablePermissionlessSplTokens == configValue);
            return tx;
        });
    }
    updateLockDuration(lockDuration) {
        return __awaiter(this, void 0, void 0, function* () {
            if (this.merkleTreeAuthorityPda == undefined) {
                yield this.getMerkleTreeAuthorityPda();
            }
            const tx = yield this.merkleTreeProgram.methods.updateLockDuration(new anchor.BN(lockDuration.toString())).accounts(Object.assign({ authority: this.payer.publicKey, merkleTreeAuthorityPda: this.merkleTreeAuthorityPda, merkleTree: this.merkleTreePubkey }, constants_1.DEFAULT_PROGRAMS))
                .signers([this.payer])
                .rpc(constants_1.confirmConfig);
            let merkleTree = yield this.merkleTreeProgram.account.merkleTree.fetch(this.merkleTreePubkey);
            (0, chai_1.assert)(merkleTree.lockDuration == lockDuration);
            console.log("lock duration updated to: ", lockDuration);
            return tx;
        });
    }
    getRegisteredVerifierPda(verifierPubkey) {
        return __awaiter(this, void 0, void 0, function* () {
            // TODO: add check whether already exists
            this.registeredVerifierPdas.push({ registeredVerifierPda: (yield web3_js_1.PublicKey.findProgramAddress([verifierPubkey.toBuffer()], this.merkleTreeProgram.programId))[0],
                verifierPubkey: verifierPubkey });
            return this.registeredVerifierPdas[this.registeredVerifierPdas.length - 1];
        });
    }
    registerVerifier(verifierPubkey) {
        return __awaiter(this, void 0, void 0, function* () {
            let registeredVerifierPda = this.registeredVerifierPdas.filter((item) => { return item.verifierPubkey === verifierPubkey; })[0];
            if (!registeredVerifierPda) {
                registeredVerifierPda = yield this.getRegisteredVerifierPda(verifierPubkey);
            }
            const tx = yield this.merkleTreeProgram.methods.registerVerifier(verifierPubkey).accounts(Object.assign({ registeredVerifierPda: registeredVerifierPda.registeredVerifierPda, authority: this.payer.publicKey, merkleTreeAuthorityPda: this.merkleTreeAuthorityPda }, constants_1.DEFAULT_PROGRAMS))
                .signers([this.payer])
                .rpc(constants_1.confirmConfig);
            yield this.checkVerifierIsRegistered(verifierPubkey);
            return tx;
        });
    }
    checkVerifierIsRegistered(verifierPubkey) {
        return __awaiter(this, void 0, void 0, function* () {
            let registeredVerifierPda = this.registeredVerifierPdas.filter((item) => { return item.verifierPubkey === verifierPubkey; })[0];
            var registeredVerifierAccountInfo = yield this.merkleTreeProgram.account.registeredVerifier.fetch(registeredVerifierPda.registeredVerifierPda);
            (0, chai_1.assert)(registeredVerifierAccountInfo != null);
            (0, chai_1.assert)(registeredVerifierAccountInfo.pubkey.toBase58() == verifierPubkey.toBase58());
        });
    }
    getPoolTypePda(poolType) {
        return __awaiter(this, void 0, void 0, function* () {
            if (poolType.length != 32) {
                throw `invalid pooltype length ${poolType.length}`;
            }
            // TODO: add check whether already exists
            this.poolTypes.push({ poolPda: (yield web3_js_1.PublicKey.findProgramAddress([poolType, anchor.utils.bytes.utf8.encode("pooltype")], this.merkleTreeProgram.programId))[0], poolType: poolType });
            return this.poolTypes[this.poolTypes.length - 1];
        });
    }
    registerPoolType(poolType) {
        return __awaiter(this, void 0, void 0, function* () {
            let registeredPoolTypePda = this.poolTypes.filter((item) => { return item.poolType === poolType; })[0];
            if (!registeredPoolTypePda) {
                registeredPoolTypePda = yield this.getPoolTypePda(poolType);
            }
            const tx = yield this.merkleTreeProgram.methods.registerPoolType(Buffer.from(new Uint8Array(32).fill(0))).accounts(Object.assign({ registeredPoolTypePda: registeredPoolTypePda.poolPda, authority: this.payer.publicKey, merkleTreeAuthorityPda: this.merkleTreeAuthorityPda }, constants_1.DEFAULT_PROGRAMS))
                .signers([this.payer])
                .rpc(constants_1.confirmConfig);
            return tx;
        });
    }
    checkPoolRegistered(poolPda, poolType, mint = null) {
        return __awaiter(this, void 0, void 0, function* () {
            var registeredTokenConfigAccount = yield this.merkleTreeProgram.account.registeredAssetPool.fetch(poolPda.pda);
            var merkleTreeAuthorityPdaAccountInfo = yield this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda);
            (0, chai_1.assert)(registeredTokenConfigAccount.poolType.toString() == poolType.toString());
            (0, chai_1.assert)(registeredTokenConfigAccount.index.toString() == (merkleTreeAuthorityPdaAccountInfo.registeredAssetIndex - 1).toString());
            if (mint !== null) {
                (0, chai_1.assert)(registeredTokenConfigAccount.assetPoolPubkey.toBase58() == poolPda.token.toBase58());
                var registeredTokenAccount = yield token.getAccount(this.connection, poolPda.token, { commitment: "confirmed", preflightCommitment: 'confirmed' });
                (0, chai_1.assert)(registeredTokenAccount != null);
                (0, chai_1.assert)(registeredTokenAccount.mint.toBase58() == mint.toBase58());
            }
            else {
                (0, chai_1.assert)(registeredTokenConfigAccount.assetPoolPubkey.toBase58() == poolPda.pda.toBase58());
            }
        });
    }
    getSolPoolPda(poolType) {
        return __awaiter(this, void 0, void 0, function* () {
            this.poolPdas.push({ pda: (yield web3_js_1.PublicKey.findProgramAddress([new Uint8Array(32).fill(0), poolType, anchor.utils.bytes.utf8.encode("pool-config")], this.merkleTreeProgram.programId))[0], poolType: poolType });
            return this.poolPdas[this.poolPdas.length - 1];
        });
    }
    registerSolPool(poolType) {
        return __awaiter(this, void 0, void 0, function* () {
            let registeredPoolTypePda = this.poolTypes.filter((item) => { return item.poolType === poolType; })[0];
            if (!registeredPoolTypePda) {
                registeredPoolTypePda = yield this.getPoolTypePda(poolType);
            }
            let solPoolPda = yield this.getSolPoolPda(poolType);
            const tx = yield this.merkleTreeProgram.methods.registerSolPool().accounts(Object.assign({ registeredAssetPoolPda: solPoolPda.pda, authority: this.payer.publicKey, merkleTreeAuthorityPda: this.merkleTreeAuthorityPda, registeredPoolTypePda: registeredPoolTypePda.poolPda }, constants_1.DEFAULT_PROGRAMS))
                .signers([this.payer])
                .rpc({ commitment: "confirmed", preflightCommitment: 'confirmed' });
            yield this.checkPoolRegistered(solPoolPda, poolType);
            console.log("registered sol pool ", this.merkleTreeAuthorityPda.toBase58());
            return tx;
        });
    }
    getSplPoolPdaToken(poolType, mint) {
        return __awaiter(this, void 0, void 0, function* () {
            let pda = (yield web3_js_1.PublicKey.findProgramAddress([mint.toBytes(), new Uint8Array(32).fill(0), anchor.utils.bytes.utf8.encode("pool")], this.merkleTreeProgram.programId))[0];
            return pda;
        });
    }
    getSplPoolPda(poolType, mint) {
        return __awaiter(this, void 0, void 0, function* () {
            this.poolPdas.push({ pda: (yield web3_js_1.PublicKey.findProgramAddress([mint.toBytes(), new Uint8Array(32).fill(0), anchor.utils.bytes.utf8.encode("pool-config")], this.merkleTreeProgram.programId))[0], poolType: poolType, token: yield this.getSplPoolPdaToken(poolType, mint) });
            return this.poolPdas[this.poolPdas.length - 1];
        });
    }
    getTokenAuthority() {
        return __awaiter(this, void 0, void 0, function* () {
            this.tokenAuthority = (yield web3_js_1.PublicKey.findProgramAddress([anchor.utils.bytes.utf8.encode("spl")], this.merkleTreeProgram.programId))[0];
            return this.tokenAuthority;
        });
    }
    registerSplPool(poolType, mint) {
        return __awaiter(this, void 0, void 0, function* () {
            let registeredPoolTypePda = this.poolTypes.filter((item) => { return item.poolType === poolType; })[0];
            if (!registeredPoolTypePda) {
                registeredPoolTypePda = yield this.getPoolTypePda(poolType);
            }
            let splPoolPda = yield this.getSplPoolPda(poolType, mint);
            if (!this.tokenAuthority) {
                yield this.getTokenAuthority();
            }
            const tx = yield this.merkleTreeProgram.methods.registerSplPool().accounts(Object.assign({ registeredAssetPoolPda: splPoolPda.pda, authority: this.payer.publicKey, merkleTreeAuthorityPda: this.merkleTreeAuthorityPda, registeredPoolTypePda: registeredPoolTypePda.poolPda, merkleTreePdaToken: splPoolPda.token, tokenAuthority: this.tokenAuthority, mint }, constants_1.DEFAULT_PROGRAMS))
                .signers([this.payer])
                .rpc(constants_1.confirmConfig);
            yield this.checkPoolRegistered(splPoolPda, poolType, mint);
            return tx;
        });
    }
}
exports.MerkleTreeConfig = MerkleTreeConfig;
