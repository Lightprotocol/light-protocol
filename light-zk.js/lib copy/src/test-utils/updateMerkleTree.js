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
Object.defineProperty(exports, "__esModule", { value: true });
exports.updateMerkleTreeForTest = void 0;
const anchor = __importStar(require("@coral-xyz/anchor"));
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