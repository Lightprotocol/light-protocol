"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.checkNfInserted = exports.checkRentExemption = exports.checkMerkleTreeBatchUpdateSuccess = exports.checkMerkleTreeUpdateStateCreated = void 0;
const tslib_1 = require("tslib");
const solana = require("@solana/web3.js");
const chai_1 = require("chai");
const anchor = tslib_1.__importStar(require("@coral-xyz/anchor"));
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
async function checkMerkleTreeUpdateStateCreated({ connection, merkleTreeUpdateState, relayer, transactionMerkleTree, leavesPdas, current_instruction_index, merkleTreeProgram, }) {
    let x = console.log;
    console.log = () => { };
    var merkleTreeTmpAccountInfo = await connection.getAccountInfo(merkleTreeUpdateState, "confirmed");
    if (!merkleTreeTmpAccountInfo)
        throw new Error("merkleTreeTmpAccountInfo null");
    chai_1.assert.equal(merkleTreeTmpAccountInfo.owner.toBase58(), merkleTreeProgram.programId.toBase58(), "merkle tree pda owner wrong after initializing");
    const merkleTreeUpdateStateData = await merkleTreeProgram.account.merkleTreeUpdateState.fetch(merkleTreeUpdateState, "confirmed");
    // merkleTreeProgram.account.preInsertedLeavesIndex._coder.accounts.decode(
    //   "MerkleTreeUpdateState",
    //   merkleTreeTmpAccountInfo.data,
    // );
    var MerkleTreeAccountInfo = await merkleTreeProgram.account.transactionMerkleTree.fetch(transactionMerkleTree, "confirmed");
    // console.log("merkleTreeUpdateStateData.leaves ", merkleTreeUpdateStateData.leaves);
    console.log("merkleTreeUpdateStateData.numberOfLeaves ", merkleTreeUpdateStateData.numberOfLeaves);
    console.log("leavesPdas.length ", leavesPdas.length);
    console.log("merkleTreeUpdateStateData.currentInstructionIndex ", merkleTreeUpdateStateData.currentInstructionIndex);
    console.log("current_instruction_index ", current_instruction_index);
    chai_1.assert.equal(merkleTreeUpdateStateData.relayer.toBase58(), relayer.toBase58(), "The incorrect signer has been saved");
    chai_1.assert.equal(merkleTreeUpdateStateData.merkleTreePdaPubkey.toBase58(), transactionMerkleTree.toBase58(), "the incorrect merkle tree pubkey was saved");
    chai_1.assert.equal(merkleTreeUpdateStateData.numberOfLeaves, leavesPdas.length, "The incorrect number of leaves was saved");
    (0, chai_1.assert)(merkleTreeUpdateStateData.currentInstructionIndex.eq(new anchor.BN(current_instruction_index)), "The instruction index is wrong");
    chai_1.assert.equal(MerkleTreeAccountInfo.pubkeyLocked.toBase58(), merkleTreeUpdateState.toBase58());
    console.log("checkMerkleTreeUpdateStateCreated: success");
    console.log = x;
}
exports.checkMerkleTreeUpdateStateCreated = checkMerkleTreeUpdateStateCreated;
async function checkMerkleTreeBatchUpdateSuccess({ connection, merkleTreeUpdateState, merkleTreeAccountPrior, numberOfLeaves, transactionMerkleTree, merkleTreeProgram, }) {
    var merkleTreeTmpStateAccount = await connection.getAccountInfo(merkleTreeUpdateState, "confirmed");
    chai_1.assert.equal(merkleTreeTmpStateAccount, null, "Shielded transaction failed merkleTreeTmpStateAccount is not closed");
    var merkleTreeAccount = await merkleTreeProgram.account.transactionMerkleTree.fetch(transactionMerkleTree, "confirmed");
    // Merkle tree is locked by merkleTreeUpdateState
    chai_1.assert.equal(merkleTreeAccount.pubkeyLocked.toBase58(), new solana.PublicKey(new Uint8Array(32).fill(0)).toBase58());
    chai_1.assert.equal(merkleTreeAccount.timeLocked.toNumber(), 0, "Lock has not been taken within prior  20 slots");
    let merkle_tree_prior_leaves_index = merkleTreeAccountPrior.nextIndex;
    let merkle_tree_prior_current_root_index = merkleTreeAccountPrior.currentRootIndex;
    let current_root_index = merkleTreeAccount.currentRootIndex;
    (0, chai_1.assert)(merkle_tree_prior_current_root_index
        .add(new anchor.BN("1"))
        .mod(new anchor.BN(256))
        .eq(current_root_index));
    (0, chai_1.assert)(merkle_tree_prior_leaves_index
        .add(new anchor.BN(numberOfLeaves))
        .eq(merkleTreeAccount.nextIndex));
}
exports.checkMerkleTreeBatchUpdateSuccess = checkMerkleTreeBatchUpdateSuccess;
async function checkRentExemption({ connection, account, }) {
    let requiredBalance = await connection.getMinimumBalanceForRentExemption(account.data.length);
    if (account.lamports < requiredBalance) {
        throw Error(`Account of size ${account.data.length} not rentexempt balance ${account.lamports} should be${requiredBalance}`);
    }
}
exports.checkRentExemption = checkRentExemption;
async function checkNfInserted(pubkeys, connection, returnValue = false) {
    for (var i = 0; i < pubkeys.length; i++) {
        var accountInfo = await connection.getAccountInfo(pubkeys[i].pubkey);
        if (!returnValue)
            chai_1.assert.equal(accountInfo, null);
        else
            return accountInfo;
    }
}
exports.checkNfInserted = checkNfInserted;
//# sourceMappingURL=testChecks.js.map