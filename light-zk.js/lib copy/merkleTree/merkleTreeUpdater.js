"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.executeMerkleTreeUpdateTransactions = exports.executeUpdateMerkleTreeTransactions = void 0;
const tslib_1 = require("tslib");
const anchor = tslib_1.__importStar(require("@coral-xyz/anchor"));
const spl_account_compression_1 = require("@solana/spl-account-compression");
const testChecks_1 = require("../test-utils/testChecks");
const index_1 = require("../index");
const web3_js_1 = require("@solana/web3.js");
async function executeUpdateMerkleTreeTransactions({ signer, merkleTreeProgram, leavesPdas, transactionMerkleTree, connection, }) {
    var merkleTreeAccountPrior = await merkleTreeProgram.account.transactionMerkleTree.fetch(transactionMerkleTree);
    let merkleTreeUpdateState = (await web3_js_1.PublicKey.findProgramAddressSync([
        Buffer.from(new Uint8Array(signer.publicKey.toBytes())),
        anchor.utils.bytes.utf8.encode("storage"),
    ], merkleTreeProgram.programId))[0];
    try {
        const tx1 = await merkleTreeProgram.methods
            .initializeMerkleTreeUpdateState()
            .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            systemProgram: web3_js_1.SystemProgram.programId,
            rent: index_1.DEFAULT_PROGRAMS.rent,
            transactionMerkleTree: transactionMerkleTree,
        })
            .remainingAccounts(leavesPdas)
            .preInstructions([
            web3_js_1.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
        ])
            .transaction();
        await (0, web3_js_1.sendAndConfirmTransaction)(connection, tx1, [signer], index_1.confirmConfig);
    }
    catch (err) {
        console.error("failed while initing the merkle tree update state", err);
        throw err;
    }
    await (0, testChecks_1.checkMerkleTreeUpdateStateCreated)({
        connection: connection,
        merkleTreeUpdateState,
        transactionMerkleTree: transactionMerkleTree,
        relayer: signer.publicKey,
        leavesPdas,
        current_instruction_index: 1,
        merkleTreeProgram,
    });
    await executeMerkleTreeUpdateTransactions({
        signer,
        merkleTreeProgram,
        transactionMerkleTree: transactionMerkleTree,
        merkleTreeUpdateState,
        connection,
    });
    await (0, testChecks_1.checkMerkleTreeUpdateStateCreated)({
        connection: connection,
        merkleTreeUpdateState,
        transactionMerkleTree: transactionMerkleTree,
        relayer: signer.publicKey,
        leavesPdas,
        current_instruction_index: 56,
        merkleTreeProgram,
    });
    try {
        const tx1 = await merkleTreeProgram.methods
            .insertRootMerkleTree(new anchor.BN(254))
            .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            transactionMerkleTree: transactionMerkleTree,
            logWrapper: spl_account_compression_1.SPL_NOOP_ADDRESS,
        })
            .remainingAccounts(leavesPdas)
            .preInstructions([
            web3_js_1.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }),
        ])
            .transaction();
        await (0, web3_js_1.sendAndConfirmTransaction)(connection, tx1, [signer], index_1.confirmConfig);
    }
    catch (e) {
        console.log(e);
        throw e;
    }
    await (0, testChecks_1.checkMerkleTreeBatchUpdateSuccess)({
        connection: connection,
        merkleTreeUpdateState: merkleTreeUpdateState,
        merkleTreeAccountPrior,
        numberOfLeaves: leavesPdas.length * 2,
        leavesPdas,
        transactionMerkleTree: transactionMerkleTree,
        merkleTreeProgram,
    });
}
exports.executeUpdateMerkleTreeTransactions = executeUpdateMerkleTreeTransactions;
const createTransactions = async ({ counter, merkleTreeProgram, numberOfTransactions, signer, merkleTreeUpdateState, transactionMerkleTree, }) => {
    let transactions = [];
    for (let ix_id = 0; ix_id < numberOfTransactions; ix_id++) {
        const transaction = new web3_js_1.Transaction();
        transaction.add(web3_js_1.ComputeBudgetProgram.setComputeUnitLimit({ units: 1400000 }));
        transaction.add(await merkleTreeProgram.methods
            .updateTransactionMerkleTree(new anchor.BN(counter.value))
            .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState,
            transactionMerkleTree: transactionMerkleTree,
        })
            .instruction());
        counter.value += 1;
        transaction.add(await merkleTreeProgram.methods
            .updateTransactionMerkleTree(new anchor.BN(counter.value))
            .accounts({
            authority: signer.publicKey,
            merkleTreeUpdateState: merkleTreeUpdateState,
            transactionMerkleTree: transactionMerkleTree,
        })
            .instruction());
        counter.value += 1;
        transactions.push(transaction);
    }
    return transactions;
};
const checkComputeInstructionsCompleted = async (merkleTreeProgram, merkleTreeUpdateState) => {
    const accountInfo = await merkleTreeProgram.account.merkleTreeUpdateState.fetch(merkleTreeUpdateState);
    return accountInfo.currentInstructionIndex.toNumber() === 56;
};
const sendAndConfirmTransactions = async (transactions, signer, connection) => {
    const errors = [];
    await Promise.all(transactions.map(async (tx) => {
        try {
            await (0, web3_js_1.sendAndConfirmTransaction)(connection, tx, [signer], index_1.confirmConfig);
        }
        catch (err) {
            errors.push(err);
        }
    }));
    if (errors.length > 0)
        throw errors[0];
};
/**
 * executeMerkleTreeUpdateTransactions attempts to execute a Merkle tree update.
 *
 * Strategy Overview:
 * - Sends an initial batch of 28 transactions including two compute instructions each.
 * - Checks if the update is complete.
 * - If not, sends additional batches of 10 transactions.
 * - This continues until the update is complete or a maximum retry limit of 240 instructions (120 transactions) is reached.
 *
 * @param {object} {
 *   merkleTreeProgram: Program<MerkleTreeProgram>,  // The Merkle tree program anchor instance.
 *   merkleTreeUpdateState: PublicKey,              // The public key of the temporary update state of the Merkle tree.
 *   transactionMerkleTree: PublicKey,              // The public key of the transaction Merkle tree which is updated.
 *   signer: Keypair,                               // The keypair used to sign the transactions.
 *   connection: Connection,                        // The network connection object.
 *   numberOfTransactions: number = 28,             // (optional) Initial number of transactions to send. Default is 28.
 *   interrupt: boolean = false                     // (optional) If true, interrupts the process. Default is false.
 * } - The input parameters for the function.
 *
 * @returns {Promise<void>} A promise that resolves when the update is complete or the maximum retry limit is reached.
 * @throws {Error} If an issue occurs while sending and confirming transactions.
 */
async function executeMerkleTreeUpdateTransactions({ merkleTreeProgram, merkleTreeUpdateState, transactionMerkleTree, signer, connection, numberOfTransactions = 56 / 2, interrupt = false, }) {
    var counter = { value: 0 };
    var error = undefined;
    while (!(await checkComputeInstructionsCompleted(merkleTreeProgram, merkleTreeUpdateState))) {
        numberOfTransactions = counter.value == 0 ? numberOfTransactions : 10;
        if (counter.value != 0)
            await (0, index_1.sleep)(1000);
        const transactions = await createTransactions({
            numberOfTransactions,
            signer,
            counter,
            merkleTreeProgram,
            merkleTreeUpdateState,
            transactionMerkleTree,
        });
        try {
            await sendAndConfirmTransactions(transactions, signer, connection);
        }
        catch (err) {
            error = err;
        }
        if (interrupt || counter.value >= 240) {
            console.log("Reached retry limit of 240 compute instructions");
            if (error)
                throw error;
            else
                return;
        }
    }
}
exports.executeMerkleTreeUpdateTransactions = executeMerkleTreeUpdateTransactions;
//# sourceMappingURL=merkleTreeUpdater.js.map