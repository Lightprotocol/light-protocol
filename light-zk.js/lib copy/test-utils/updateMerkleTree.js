"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.updateMerkleTreeForTest = void 0;
const tslib_1 = require("tslib");
const anchor = tslib_1.__importStar(require("@coral-xyz/anchor"));
const index_1 = require("../merkleTree/index");
const constants_1 = require("../constants");
const index_2 = require("../idls/index");
const web3_js_1 = require("@solana/web3.js");
const index_3 = require("../index");
async function updateMerkleTreeForTest(payer, url) {
    const connection = new web3_js_1.Connection(url, constants_1.confirmConfig);
    const anchorProvider = new anchor.AnchorProvider(connection, new anchor.Wallet(web3_js_1.Keypair.generate()), constants_1.confirmConfig);
    try {
        const merkleTreeProgram = new anchor.Program(index_2.IDL_MERKLE_TREE_PROGRAM, constants_1.merkleTreeProgramId, anchorProvider && anchorProvider);
        const transactionMerkleTreePda = index_1.MerkleTreeConfig.getTransactionMerkleTreePda();
        let leavesPdas = [];
        let retries = 5;
        while (leavesPdas.length === 0 && retries > 0) {
            if (retries !== 5)
                await (0, index_3.sleep)(1000);
            leavesPdas = await index_1.SolMerkleTree.getUninsertedLeavesRelayer(transactionMerkleTreePda, anchorProvider && anchorProvider);
            retries--;
        }
        await (0, index_1.executeUpdateMerkleTreeTransactions)({
            connection,
            signer: payer,
            merkleTreeProgram,
            leavesPdas,
            transactionMerkleTree: transactionMerkleTreePda,
        });
    }
    catch (err) {
        console.error("failed at updateMerkleTreeForTest", err);
        throw err;
    }
}
exports.updateMerkleTreeForTest = updateMerkleTreeForTest;
//# sourceMappingURL=updateMerkleTree.js.map