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
exports.checkNfInserted = exports.checkRentExemption = exports.checkMerkleTreeBatchUpdateSuccess = exports.checkMerkleTreeUpdateStateCreated = void 0;
const solana = require("@solana/web3.js");
const chai_1 = require("chai");
const token = require("@solana/spl-token");
const anchor = __importStar(require("@coral-xyz/anchor"));
/*
 *
 * Checks:
 * owner
 * relayer saved as signer
 * merkle tree saved
 * number of leaves saved correctly
 * current instruction index is correct
 * merkle tree is locked by updateState account
 * lock has been taken less than 5 slots ago
 */
let CONFIRMATION = {
    preflightCommitment: "confirmed",
    commitment: "confirmed",
};
function checkMerkleTreeUpdateStateCreated({ connection, merkleTreeUpdateState, relayer, MerkleTree, leavesPdas, current_instruction_index, merkleTreeProgram, }) {
    return __awaiter(this, void 0, void 0, function* () {
        var merkleTreeTmpAccountInfo = yield connection.getAccountInfo(merkleTreeUpdateState, CONFIRMATION);
        (0, chai_1.assert)(merkleTreeTmpAccountInfo.owner.toBase58() ===
            merkleTreeProgram.programId.toBase58(), "merkle tree pda owner wrong after initializing");
        const merkleTreeUpdateStateData = yield merkleTreeProgram.account.merkleTreeUpdateState.fetch(merkleTreeUpdateState);
        // merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode(
        //   "MerkleTreeUpdateState",
        //   merkleTreeTmpAccountInfo.data,
        // );
        var MerkleTreeAccountInfo = yield merkleTreeProgram.account.merkleTree.fetch(MerkleTree);
        // console.log("merkleTreeUpdateStateData.leaves ", merkleTreeUpdateStateData.leaves);
        console.log("merkleTreeUpdateStateData.numberOfLeaves ", merkleTreeUpdateStateData.numberOfLeaves);
        console.log("leavesPdas.length ", leavesPdas.length);
        console.log("merkleTreeUpdateStateData.currentInstructionIndex ", merkleTreeUpdateStateData.currentInstructionIndex);
        console.log("current_instruction_index ", current_instruction_index);
        (0, chai_1.assert)(merkleTreeUpdateStateData.relayer.toBase58() == relayer.toBase58(), "The incorrect signer has been saved");
        (0, chai_1.assert)(merkleTreeUpdateStateData.merkleTreePdaPubkey.toBase58() ==
            MerkleTree.toBase58(), "the incorrect merkle tree pubkey was saved");
        (0, chai_1.assert)(merkleTreeUpdateStateData.numberOfLeaves == leavesPdas.length, "The incorrect number of leaves was saved");
        (0, chai_1.assert)(merkleTreeUpdateStateData.currentInstructionIndex ==
            current_instruction_index, "The instruction index is wrong");
        (0, chai_1.assert)(MerkleTreeAccountInfo.pubkeyLocked.toBase58() ==
            merkleTreeUpdateState.toBase58());
        // assert(U64.readLE(MerkleTreeAccountInfo.data.slice(16658-8,16658), 0) >= (await connection.getSlot()) - 5, "Lock has not been taken at this or in the 5 prior slots");
        console.log("checkMerkleTreeUpdateStateCreated: success");
    });
}
exports.checkMerkleTreeUpdateStateCreated = checkMerkleTreeUpdateStateCreated;
function checkMerkleTreeBatchUpdateSuccess({ connection, merkleTreeUpdateState, merkleTreeAccountPrior, numberOfLeaves, leavesPdas, merkleTree, merkle_tree_pubkey, merkleTreeProgram, }) {
    return __awaiter(this, void 0, void 0, function* () {
        var merkleTreeTmpStateAccount = yield connection.getAccountInfo(merkleTreeUpdateState);
        chai_1.assert.equal(merkleTreeTmpStateAccount, null, "Shielded transaction failed merkleTreeTmpStateAccount is not closed");
        var merkleTreeAccount = yield merkleTreeProgram.account.merkleTree.fetch(merkle_tree_pubkey);
        // Merkle tree is locked by merkleTreeUpdateState
        chai_1.assert.equal(merkleTreeAccount.pubkeyLocked.toBase58(), new solana.PublicKey(new Uint8Array(32).fill(0)).toBase58());
        console.log("merkleTreeAccount.time_locked ", merkleTreeAccount.timeLocked);
        chai_1.assert.equal(merkleTreeAccount.timeLocked, 0, "Lock has not been taken within prior  20 slots");
        let merkle_tree_prior_leaves_index = merkleTreeAccountPrior.nextIndex; //U64.readLE(merkleTreeAccountPrior.data.slice(594, 594 + 8),0);
        let merkle_tree_prior_current_root_index = merkleTreeAccountPrior.currentRootIndex; //U64.readLE(merkleTreeAccountPrior.data.slice(594 - 8, 594),0).toNumber()
        let current_root_index = merkleTreeAccount.currentRootIndex; //tU64.readLE(merkleTreeAccount.data.slice(594 - 8, 594),0).toNumber()
        console.log("merkle_tree_prior_current_root_index: ", merkle_tree_prior_current_root_index);
        console.log("current_root_index: ", current_root_index);
        console.log(`${merkle_tree_prior_current_root_index.add(new anchor.BN("1"))} == ${current_root_index}`);
        chai_1.assert.equal(merkle_tree_prior_current_root_index.add(new anchor.BN("1")).toString(), current_root_index.toString());
        let current_root_start_range = 610 + current_root_index * 32;
        let current_root_end_range = 610 + (current_root_index + 1) * 32;
        // console.log(`root: ${BigNumber.from(merkleTreeAccount.data.slice(current_root_start_range, current_root_end_range).reverse()).toHexString()}`)
        // console.log(`prior +${numberOfLeaves} ${merkle_tree_prior_leaves_index.add(new anchor.BN(numberOfLeaves)).toString()}, now ${U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString()}`)
        // // index has increased by numberOfLeaves
        // console.log(`index has increased by numberOfLeaves: ${merkle_tree_prior_leaves_index.add(U64(numberOfLeaves)).toString()}, ${U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString()}`)
        console.log(`${merkle_tree_prior_leaves_index.add(new anchor.BN(numberOfLeaves))} == ${merkleTreeAccount.nextIndex}`);
        (0, chai_1.assert)(merkle_tree_prior_leaves_index
            .add(new anchor.BN(numberOfLeaves))
            .toString() == merkleTreeAccount.nextIndex.toString()); //U64.readLE(merkleTreeAccount.data.slice(594, 594 + 8), 0).toString())
        // let leavesPdasPubkeys = []
        // leavesPdas.map( (pda) => { leavesPdasPubkeys.push(pda.pubkey) })
        // var leavesAccounts = await merkleTreeProgram.account.twoLeavesBytesPda.fetchMultiple(
        //   leavesPdasPubkeys
        //     )
        // let leaves_to_sort = []
        // leavesAccounts.map((acc) => {
        //   // Checking that all leaves have been marked as inserted.
        //   assert(leavesAccounts.isInserted == true);
        //     leaves_to_sort.push(leavesAccounts);
        //   });
        // leaves_to_sort.sort((a, b) => parseFloat(a.left_leaf_index) - parseFloat(b.left_leaf_index));
        // let numberOfLeavesPdas = 0
        // for (var i = Number(merkle_tree_prior_leaves_index); i < Number(merkle_tree_prior_leaves_index) + Number(numberOfLeaves); i+=2) {
        //   merkleTree.update(i, BigNumber.from(leaves_to_sort[numberOfLeavesPdas].leaves.slice(0,32).reverse()))
        //   merkleTree.update(i + 1, BigNumber.from(leaves_to_sort[numberOfLeavesPdas].leaves.slice(32,64).reverse()))
        //   numberOfLeavesPdas++;
        // }
        //
        // // Comparing root from chain with locally updated merkle tree.
        // assert(BigNumber.from(merkleTreeAccount.data.slice(current_root_start_range, current_root_end_range).reverse()).toHexString(),
        //   merkleTree.root().toHexString()
        // )
        // // Comparing locally generated root with merkle tree built from leaves fetched from chain.
    });
}
exports.checkMerkleTreeBatchUpdateSuccess = checkMerkleTreeBatchUpdateSuccess;
function checkRentExemption({ connection, account }) {
    return __awaiter(this, void 0, void 0, function* () {
        let requiredBalance = yield connection.getMinimumBalanceForRentExemption(account.data.length);
        if (account.lamports < requiredBalance) {
            throw Error(`Account of size ${account.data.length} not rentexempt balance ${account.lamports} should be${requiredBalance}`);
        }
    });
}
exports.checkRentExemption = checkRentExemption;
function checkNfInserted(pubkeys, connection) {
    return __awaiter(this, void 0, void 0, function* () {
        for (var i = 0; i < pubkeys.length; i++) {
            var accountInfo = yield connection.getAccountInfo(pubkeys[i]);
            chai_1.assert.equal(accountInfo, null);
        }
    });
}
exports.checkNfInserted = checkNfInserted;
