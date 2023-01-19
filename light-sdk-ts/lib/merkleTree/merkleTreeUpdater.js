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
exports.executeMerkleTreeUpdateTransactions = exports.executeUpdateMerkleTreeTransactions = void 0;
const anchor = __importStar(require("@coral-xyz/anchor"));
const testChecks_1 = require("../test-utils/testChecks");
const index_1 = require("../index");
const web3_js_1 = require("@solana/web3.js");
function executeUpdateMerkleTreeTransactions({ signer, merkleTreeProgram, leavesPdas, merkleTree, merkleTreeIndex, merkle_tree_pubkey, connection, provider, }) {
    return __awaiter(this, void 0, void 0, function* () {
        var merkleTreeAccountPrior = yield merkleTreeProgram.account.merkleTree.fetch(merkle_tree_pubkey);
        let merkleTreeUpdateState = (yield web3_js_1.PublicKey.findProgramAddressSync([
            Buffer.from(new Uint8Array(signer.publicKey.toBytes())),
            anchor.utils.bytes.utf8.encode("storage"),
        ], merkleTreeProgram.programId))[0];
        try {
            const tx1 = yield merkleTreeProgram.methods
                .initializeMerkleTreeUpdateState()
                .accounts({
                authority: signer.publicKey,
                merkleTreeUpdateState: merkleTreeUpdateState,
                systemProgram: web3_js_1.SystemProgram.programId,
                rent: index_1.DEFAULT_PROGRAMS.rent,
                merkleTree: merkle_tree_pubkey,
            })
                .remainingAccounts(leavesPdas)
                .preInstructions([
                web3_js_1.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
            ])
                .signers([signer])
                .rpc({
                commitment: "confirmed",
                preflightCommitment: "confirmed",
            });
        }
        catch (e) {
            console.log(" init Merkle tree update", e);
        }
        yield (0, testChecks_1.checkMerkleTreeUpdateStateCreated)({
            connection: connection,
            merkleTreeUpdateState,
            MerkleTree: merkle_tree_pubkey,
            relayer: signer.publicKey,
            leavesPdas,
            current_instruction_index: 1,
            merkleTreeProgram,
        });
        yield executeMerkleTreeUpdateTransactions({
            signer,
            merkleTreeProgram,
            merkle_tree_pubkey,
            provider,
            merkleTreeUpdateState,
            numberOfTransactions: 251,
        });
        yield (0, testChecks_1.checkMerkleTreeUpdateStateCreated)({
            connection: connection,
            merkleTreeUpdateState,
            MerkleTree: merkle_tree_pubkey,
            relayer: signer.publicKey,
            leavesPdas,
            current_instruction_index: 56,
            merkleTreeProgram,
        });
        // final tx to insert root
        let success = false;
        try {
            yield merkleTreeProgram.methods
                .insertRootMerkleTree(new anchor.BN(254))
                .accounts({
                authority: signer.publicKey,
                merkleTreeUpdateState: merkleTreeUpdateState,
                merkleTree: merkle_tree_pubkey,
            })
                .remainingAccounts(leavesPdas)
                .signers([signer])
                .rpc({
                commitment: "confirmed",
                preflightCommitment: "confirmed",
            });
        }
        catch (e) {
            console.log(e);
        }
        yield (0, testChecks_1.checkMerkleTreeBatchUpdateSuccess)({
            connection: connection,
            merkleTreeUpdateState: merkleTreeUpdateState,
            merkleTreeAccountPrior,
            numberOfLeaves: leavesPdas.length * 2,
            leavesPdas,
            merkleTree: merkleTree,
            merkle_tree_pubkey: merkle_tree_pubkey,
            merkleTreeProgram,
        });
    });
}
exports.executeUpdateMerkleTreeTransactions = executeUpdateMerkleTreeTransactions;
function executeMerkleTreeUpdateTransactions({ merkleTreeProgram, merkleTreeUpdateState, merkle_tree_pubkey, provider, signer, numberOfTransactions, }) {
    return __awaiter(this, void 0, void 0, function* () {
        let arr = [];
        let i = 0;
        // console.log("Sending Merkle tree update transactions: ",numberOfTransactions)
        // the number of tx needs to increase with greater batchsize
        // 29 + 2 * leavesPdas.length is a first approximation
        for (let ix_id = 0; ix_id < numberOfTransactions; ix_id++) {
            const transaction = new web3_js_1.Transaction();
            transaction.add(web3_js_1.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }));
            transaction.add(yield merkleTreeProgram.methods
                .updateMerkleTree(new anchor.BN(i))
                .accounts({
                authority: signer.publicKey,
                merkleTreeUpdateState: merkleTreeUpdateState,
                merkleTree: merkle_tree_pubkey,
            })
                .instruction());
            i += 1;
            transaction.add(yield merkleTreeProgram.methods
                .updateMerkleTree(new anchor.BN(i))
                .accounts({
                authority: signer.publicKey,
                merkleTreeUpdateState: merkleTreeUpdateState,
                merkleTree: merkle_tree_pubkey,
            })
                .instruction());
            i += 1;
            arr.push({ tx: transaction, signers: [signer] });
        }
        let error;
        yield Promise.all(arr.map((tx, index) => __awaiter(this, void 0, void 0, function* () {
            try {
                yield provider.sendAndConfirm(tx.tx, tx.signers, index_1.confirmConfig);
            }
            catch (e) {
                console.log(e);
                error = e;
            }
        })));
        return error;
    });
}
exports.executeMerkleTreeUpdateTransactions = executeMerkleTreeUpdateTransactions;
