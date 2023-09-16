"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.MerkleTreeConfig = void 0;
const tslib_1 = require("tslib");
const anchor = tslib_1.__importStar(require("@coral-xyz/anchor"));
const index_1 = require("../idls/index");
const chai_1 = require("chai");
const token = require("@solana/spl-token");
const web3_js_1 = require("@solana/web3.js");
const index_2 = require("../index");
const anchor_1 = require("@coral-xyz/anchor");
class MerkleTreeConfig {
    constructor({ payer, connection, }) {
        this.payer = payer;
        this.merkleTreeProgram = new anchor_1.Program(index_1.IDL_MERKLE_TREE_PROGRAM, index_2.merkleTreeProgramId);
        // TODO: reorg pool pdas, have one object per pool type and then an array with registered pools of this type
        this.poolPdas = [];
        this.poolTypes = [];
        this.registeredVerifierPdas = [];
        this.connection = connection;
    }
    async initializeNewTransactionMerkleTree(oldTransactionMerkleTree, newTransactionMerkleTree) {
        if (!this.payer)
            throw new Error("Payer undefined");
        this.getMerkleTreeAuthorityPda();
        const tx = await this.merkleTreeProgram.methods
            .initializeNewTransactionMerkleTree(new anchor.BN("50"))
            .accounts({
            authority: this.payer.publicKey,
            newTransactionMerkleTree: newTransactionMerkleTree,
            systemProgram: index_2.DEFAULT_PROGRAMS.systemProgram,
            rent: index_2.DEFAULT_PROGRAMS.rent,
            merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        })
            .remainingAccounts([
            {
                isSigner: false,
                isWritable: true,
                pubkey: oldTransactionMerkleTree,
            },
        ])
            .signers([this.payer])
            .transaction();
        const txHash = await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [this.payer], index_2.confirmConfig);
        await this.checkTransactionMerkleTreeIsInitialized(newTransactionMerkleTree);
        return txHash;
    }
    async checkTransactionMerkleTreeIsInitialized(transactionMerkleTreePda) {
        var transactionMerkleTreeAccountInfo = await this.merkleTreeProgram.account.transactionMerkleTree.fetch(transactionMerkleTreePda);
        (0, chai_1.assert)(transactionMerkleTreeAccountInfo != null, "merkleTreeAccountInfo not initialized");
        // zero values
        // index == 0
        // roots are empty save for 0
        // lock duration is correct
        (0, chai_1.assert)(transactionMerkleTreeAccountInfo.lockDuration.toString() == "50");
    }
    async initializeNewEventMerkleTree() {
        if (!this.payer)
            throw new Error("Payer undefined");
        await this.getMerkleTreeAuthorityPda();
        const tx = await this.merkleTreeProgram.methods
            .initializeNewEventMerkleTree()
            .accounts({
            authority: this.payer.publicKey,
            eventMerkleTree: MerkleTreeConfig.getEventMerkleTreePda(),
            merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
            systemProgram: index_2.DEFAULT_PROGRAMS.systemProgram,
        })
            .signers([this.payer])
            .transaction();
        const txHash = await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [this.payer], index_2.confirmConfig);
        await this.checkEventMerkleTreeIsInitialized();
        return txHash;
    }
    async checkEventMerkleTreeIsInitialized() {
        var merkleTreeAccountInfo = await this.merkleTreeProgram.account.eventMerkleTree.fetch(MerkleTreeConfig.getEventMerkleTreePda());
        (0, chai_1.assert)(merkleTreeAccountInfo != null, "merkleTreeAccountInfo not initialized");
    }
    async printMerkleTree() {
        var merkleTreeAccountInfo = await this.merkleTreeProgram.account.transactionMerkleTree.fetch(MerkleTreeConfig.getTransactionMerkleTreePda());
        console.log("merkleTreeAccountInfo ", merkleTreeAccountInfo);
    }
    getMerkleTreeAuthorityPda() {
        this.merkleTreeAuthorityPda = web3_js_1.PublicKey.findProgramAddressSync([anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY")], this.merkleTreeProgram.programId)[0];
        return this.merkleTreeAuthorityPda;
    }
    async getMerkleTreeAuthorityAccountInfo() {
        return await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.getMerkleTreeAuthorityPda());
    }
    async getTransactionMerkleTreeIndex() {
        let merkleTreeAuthorityAccountInfo = await this.getMerkleTreeAuthorityAccountInfo();
        return merkleTreeAuthorityAccountInfo.transactionMerkleTreeIndex;
    }
    static getTransactionMerkleTreePda(transactionMerkleTreeIndex = new anchor.BN(0)) {
        let transactionMerkleTreePda = web3_js_1.PublicKey.findProgramAddressSync([
            anchor.utils.bytes.utf8.encode("transaction_merkle_tree"),
            transactionMerkleTreeIndex.toArrayLike(Buffer, "le", 8),
        ], index_2.merkleTreeProgramId)[0];
        return transactionMerkleTreePda;
    }
    static getEventMerkleTreePda(eventMerkleTreeIndex = new anchor.BN(0)) {
        let eventMerkleTreePda = web3_js_1.PublicKey.findProgramAddressSync([
            anchor.utils.bytes.utf8.encode("event_merkle_tree"),
            eventMerkleTreeIndex.toArrayLike(Buffer, "le", 8),
        ], index_2.merkleTreeProgramId)[0];
        return eventMerkleTreePda;
    }
    async initMerkleTreeAuthority(authority, transactionMerkleTree) {
        if (authority == undefined) {
            authority = this.payer;
        }
        if (transactionMerkleTree == undefined) {
            transactionMerkleTree = MerkleTreeConfig.getTransactionMerkleTreePda(new anchor.BN(0));
        }
        if (this.merkleTreeAuthorityPda == undefined) {
            await this.getMerkleTreeAuthorityPda();
        }
        const tx = await this.merkleTreeProgram.methods
            .initializeMerkleTreeAuthority()
            .accounts({
            authority: authority === null || authority === void 0 ? void 0 : authority.publicKey,
            merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
            transactionMerkleTree: transactionMerkleTree,
            ...index_2.DEFAULT_PROGRAMS,
        })
            .signers([authority])
            .transaction();
        const txHash = await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [authority ? authority : this.payer], index_2.confirmConfig);
        // assert(
        //   this.connection.getAccountInfo(
        //     this.merkleTreeAuthorityPda!,
        //     "confirmed",
        //   ) != null,
        //   "init authority failed",
        // );
        // let merkleTreeAuthority =
        //   await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
        //     this.merkleTreeAuthorityPda!,
        //   );
        // assert(merkleTreeAuthority.enablePermissionlessSplTokens == false);
        // assert(merkleTreeAuthority.enableNfts == false);
        // assert(
        //   merkleTreeAuthority.pubkey.toBase58() == authority!.publicKey.toBase58(),
        // );
        // assert(merkleTreeAuthority.registeredAssetIndex.toString() == "0");
        return txHash;
    }
    async isMerkleTreeAuthorityInitialized() {
        const accountInfo = await this.connection.getAccountInfo(this.getMerkleTreeAuthorityPda());
        return accountInfo !== null && accountInfo.data.length >= 0;
    }
    async updateMerkleTreeAuthority(newAuthority, test = false) {
        if (!this.merkleTreeAuthorityPda) {
            await this.getMerkleTreeAuthorityPda();
        }
        if (!this.payer)
            throw new Error("Payer undefined");
        let merkleTreeAuthorityPrior = null;
        if (test != true) {
            merkleTreeAuthorityPrior =
                await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda);
            if (merkleTreeAuthorityPrior == null) {
                throw `Merkle tree authority ${this.merkleTreeAuthorityPda.toBase58()} not initialized`;
            }
        }
        const tx = await this.merkleTreeProgram.methods
            .updateMerkleTreeAuthority()
            .accounts({
            authority: this.payer.publicKey,
            newAuthority,
            merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
            ...index_2.DEFAULT_PROGRAMS,
        })
            .signers([this.payer])
            .transaction();
        const txHash = await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [this.payer], index_2.confirmConfig);
        if (test != true) {
            let merkleTreeAuthority = await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda);
            chai_1.assert.equal(merkleTreeAuthority.enablePermissionlessSplTokens, merkleTreeAuthorityPrior.enablePermissionlessSplTokens);
            chai_1.assert.equal(merkleTreeAuthority.enableNfts, merkleTreeAuthorityPrior.enableNfts);
            chai_1.assert.equal(merkleTreeAuthority.pubkey.toBase58(), newAuthority.toBase58());
        }
        return txHash;
    }
    // commented in program
    // async enableNfts(configValue: Boolean) {
    //   if (this.merkleTreeAuthorityPda == undefined) {
    //     await this.getMerkleTreeAuthorityPda();
    //   }
    //   const tx = await this.merkleTreeProgram.methods
    //     .enableNfts(configValue)
    //     .accounts({
    //       authority: this.payer.publicKey,
    //       merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
    //       ...DEFAULT_PROGRAMS,
    //     })
    //     .signers([this.payer])
    //     .rpc(confirmConfig);
    //   let merkleTreeAuthority =
    //     await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
    //       this.merkleTreeAuthorityPda,
    //     );
    //   assert(merkleTreeAuthority.enableNfts == configValue);
    //   return tx;
    // }
    async enablePermissionlessSplTokens(configValue) {
        if (!this.payer)
            throw new Error("Payer undefined");
        if (this.merkleTreeAuthorityPda == undefined) {
            await this.getMerkleTreeAuthorityPda();
        }
        const tx = await this.merkleTreeProgram.methods
            .enablePermissionlessSplTokens(configValue)
            .accounts({
            authority: this.payer.publicKey,
            merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
            ...index_2.DEFAULT_PROGRAMS,
        })
            .signers([this.payer])
            .transaction();
        const txHash = await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [this.payer], index_2.confirmConfig);
        let merkleTreeAuthority = await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda);
        (0, chai_1.assert)(merkleTreeAuthority.enablePermissionlessSplTokens == configValue);
        return txHash;
    }
    async updateLockDuration(lockDuration) {
        if (!this.payer)
            throw new Error("Payer undefined");
        if (this.merkleTreeAuthorityPda == undefined) {
            await this.getMerkleTreeAuthorityPda();
        }
        const transactionMerkleTreePda = MerkleTreeConfig.getTransactionMerkleTreePda();
        const tx = await this.merkleTreeProgram.methods
            .updateLockDuration(new anchor.BN(lockDuration.toString()))
            .accounts({
            authority: this.payer.publicKey,
            merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
            transactionMerkleTree: transactionMerkleTreePda,
            ...index_2.DEFAULT_PROGRAMS,
        })
            .signers([this.payer])
            .transaction();
        const txHash = await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [this.payer], index_2.confirmConfig);
        let merkleTree = await this.merkleTreeProgram.account.transactionMerkleTree.fetch(transactionMerkleTreePda);
        chai_1.assert.equal(merkleTree.lockDuration.toString(), lockDuration.toString());
        console.log("lock duration updated to: ", lockDuration);
        return txHash;
    }
    async getRegisteredVerifierPda(verifierPubkey) {
        // TODO: add check whether already exists
        this.registeredVerifierPdas.push({
            registeredVerifierPda: web3_js_1.PublicKey.findProgramAddressSync([verifierPubkey.toBuffer()], this.merkleTreeProgram.programId)[0],
            verifierPubkey: verifierPubkey,
        });
        return this.registeredVerifierPdas[this.registeredVerifierPdas.length - 1];
    }
    async registerVerifier(verifierPubkey) {
        if (!this.payer)
            throw new Error("Payer undefined");
        let registeredVerifierPda = this.registeredVerifierPdas.filter((item) => {
            return item.verifierPubkey === verifierPubkey;
        })[0];
        if (!registeredVerifierPda) {
            registeredVerifierPda = await this.getRegisteredVerifierPda(verifierPubkey);
        }
        const tx = await this.merkleTreeProgram.methods
            .registerVerifier(verifierPubkey)
            .accounts({
            registeredVerifierPda: registeredVerifierPda.registeredVerifierPda,
            authority: this.payer.publicKey,
            merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
            ...index_2.DEFAULT_PROGRAMS,
        })
            .signers([this.payer])
            .transaction();
        const txHash = await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [this.payer], index_2.confirmConfig);
        await this.checkVerifierIsRegistered(verifierPubkey);
        return txHash;
    }
    async checkVerifierIsRegistered(verifierPubkey) {
        let registeredVerifierPda = this.registeredVerifierPdas.filter((item) => {
            return item.verifierPubkey === verifierPubkey;
        })[0];
        var registeredVerifierAccountInfo = await this.merkleTreeProgram.account.registeredVerifier.fetch(registeredVerifierPda.registeredVerifierPda);
        (0, chai_1.assert)(registeredVerifierAccountInfo != null);
        (0, chai_1.assert)(registeredVerifierAccountInfo.pubkey.toBase58() ==
            verifierPubkey.toBase58());
    }
    async getPoolTypePda(poolType) {
        if (poolType.length != 32) {
            throw `invalid pooltype length ${poolType.length}`;
        }
        // TODO: add check whether already exists
        this.poolTypes.push({
            tokenPdas: [],
            poolPda: web3_js_1.PublicKey.findProgramAddressSync([Buffer.from(poolType), anchor.utils.bytes.utf8.encode("pooltype")], this.merkleTreeProgram.programId)[0],
            poolType: poolType,
        });
        return this.poolTypes[this.poolTypes.length - 1];
    }
    async registerPoolType(poolType) {
        if (!this.payer)
            throw new Error("Payer undefined");
        let registeredPoolTypePda = this.poolTypes.filter((item) => {
            return item.poolType.toString() === poolType.toString();
        })[0];
        if (!registeredPoolTypePda) {
            registeredPoolTypePda = await this.getPoolTypePda(poolType);
        }
        const tx = await this.merkleTreeProgram.methods
            .registerPoolType(poolType)
            .accounts({
            registeredPoolTypePda: registeredPoolTypePda.poolPda,
            authority: this.payer.publicKey,
            merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
            ...index_2.DEFAULT_PROGRAMS,
        })
            .signers([this.payer])
            .transaction();
        const txHash = await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [this.payer], index_2.confirmConfig);
        return txHash;
    }
    async checkPoolRegistered(poolPda, poolType, mint = null) {
        if (!this.merkleTreeAuthorityPda)
            throw new Error("merkleTreeAuthorityPda undefined");
        var registeredTokenConfigAccount = await this.merkleTreeProgram.account.registeredAssetPool.fetch(poolPda.pda);
        var merkleTreeAuthorityPdaAccountInfo = await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(this.merkleTreeAuthorityPda);
        chai_1.assert.equal(registeredTokenConfigAccount.poolType.toString(), poolType.toString());
        chai_1.assert.equal(registeredTokenConfigAccount.index.toString(), (merkleTreeAuthorityPdaAccountInfo.registeredAssetIndex.toNumber() - 1).toString());
        if (mint !== null) {
            chai_1.assert.equal(registeredTokenConfigAccount.assetPoolPubkey.toBase58(), poolPda.token.toBase58());
            var registeredTokenAccount = await token.getAccount(this.connection, poolPda.token, { commitment: "confirmed", preflightCommitment: "confirmed" });
            chai_1.assert.notEqual(registeredTokenAccount, null);
            chai_1.assert.equal(registeredTokenAccount.mint.toBase58(), mint.toBase58());
        }
        else {
            chai_1.assert.equal(registeredTokenConfigAccount.assetPoolPubkey.toBase58(), poolPda.pda.toBase58());
        }
    }
    static getSolPoolPda(programId, poolType = new Array(32)) {
        return {
            pda: web3_js_1.PublicKey.findProgramAddressSync([
                new Uint8Array(32).fill(0),
                Buffer.from(poolType),
                anchor.utils.bytes.utf8.encode("pool-config"),
            ], programId)[0],
            poolType: poolType,
        };
    }
    async registerSolPool(poolType) {
        if (!this.payer)
            throw new Error("Payer undefined");
        if (!this.merkleTreeAuthorityPda)
            throw new Error("merkleTreeAuthorityPda undefined");
        let registeredPoolTypePda = this.poolTypes.filter((item) => {
            return item.poolType.toString() === poolType.toString();
        })[0];
        if (!registeredPoolTypePda) {
            registeredPoolTypePda = await this.getPoolTypePda(poolType);
        }
        let solPoolPda = MerkleTreeConfig.getSolPoolPda(this.merkleTreeProgram.programId, poolType);
        const tx = await this.merkleTreeProgram.methods
            .registerSolPool()
            .accounts({
            registeredAssetPoolPda: solPoolPda.pda,
            authority: this.payer.publicKey,
            merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
            registeredPoolTypePda: registeredPoolTypePda.poolPda,
            ...index_2.DEFAULT_PROGRAMS,
        })
            .signers([this.payer])
            .transaction();
        const txHash = await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [this.payer], index_2.confirmConfig);
        await this.checkPoolRegistered(solPoolPda, poolType);
        console.log("registered sol pool ", this.merkleTreeAuthorityPda.toBase58());
        // no need to push the sol pool because it is the pool config pda
        // TODO: evaluate how to handle this case
        return txHash;
    }
    static getSplPoolPdaToken(mint, programId, poolType = new Array(32).fill(0)) {
        let pda = web3_js_1.PublicKey.findProgramAddressSync([
            mint.toBytes(),
            Buffer.from(poolType),
            anchor.utils.bytes.utf8.encode("pool"),
        ], programId)[0];
        return pda;
    }
    async getSplPoolPda(mint, poolType = new Array(32).fill(0)) {
        this.poolPdas.push({
            pda: web3_js_1.PublicKey.findProgramAddressSync([
                mint.toBytes(),
                new Uint8Array(32).fill(0),
                anchor.utils.bytes.utf8.encode("pool-config"),
            ], this.merkleTreeProgram.programId)[0],
            poolType: poolType,
            token: await MerkleTreeConfig.getSplPoolPdaToken(mint, this.merkleTreeProgram.programId, poolType),
        });
        return this.poolPdas[this.poolPdas.length - 1];
    }
    async getTokenAuthority() {
        this.tokenAuthority = web3_js_1.PublicKey.findProgramAddressSync([anchor.utils.bytes.utf8.encode("spl")], this.merkleTreeProgram.programId)[0];
        return this.tokenAuthority;
    }
    async registerSplPool(poolType, mint) {
        if (!this.payer)
            throw new Error("Payer undefined");
        let registeredPoolTypePda = this.poolTypes.filter((item) => {
            return item.poolType === poolType;
        })[0];
        if (!registeredPoolTypePda) {
            registeredPoolTypePda = await this.getPoolTypePda(poolType);
        }
        let splPoolPda = await this.getSplPoolPda(mint, poolType);
        if (!this.tokenAuthority) {
            await this.getTokenAuthority();
        }
        const tx = await this.merkleTreeProgram.methods
            .registerSplPool()
            .accounts({
            registeredAssetPoolPda: splPoolPda.pda,
            authority: this.payer.publicKey,
            merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
            registeredPoolTypePda: registeredPoolTypePda.poolPda,
            merkleTreePdaToken: splPoolPda.token,
            tokenAuthority: this.tokenAuthority,
            mint,
            ...index_2.DEFAULT_PROGRAMS,
        })
            .signers([this.payer])
            .transaction();
        const txHash = await (0, web3_js_1.sendAndConfirmTransaction)(this.connection, tx, [this.payer], index_2.confirmConfig);
        await this.checkPoolRegistered(splPoolPda, poolType, mint);
        return txHash;
    }
}
exports.MerkleTreeConfig = MerkleTreeConfig;
//# sourceMappingURL=merkleTreeConfig.js.map