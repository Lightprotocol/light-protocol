"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.SolMerkleTree = void 0;
const anchor_1 = require("@coral-xyz/anchor");
const index_1 = require("../index");
const index_2 = require("../idls/index");
const merkleTree_1 = require("./merkleTree");
const anchor = require("@coral-xyz/anchor");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;
// TODO: once we have multiple trees add merkleTree[] and fetchTree(pubkey);
class SolMerkleTree {
    constructor({ pubkey, poseidon, merkleTree = new merkleTree_1.MerkleTree(index_1.MERKLE_TREE_HEIGHT, poseidon), }) {
        this.pubkey = pubkey;
        this.merkleTree = merkleTree;
    }
    static async getLeaves(merkleTreePubkey, provider) {
        const merkleTreeProgram = new anchor_1.Program(index_2.IDL_MERKLE_TREE_PROGRAM, index_1.merkleTreeProgramId, provider);
        const mtFetched = await merkleTreeProgram.account.transactionMerkleTree.fetch(merkleTreePubkey, "processed");
        const merkleTreeIndex = mtFetched.nextIndex;
        // ProgramAccount<MerkleTreeProgram["accounts"][7]>
        var leavesAccounts = await merkleTreeProgram.account.twoLeavesBytesPda.all();
        return { leavesAccounts, merkleTreeIndex, mtFetched };
    }
    static async build({ pubkey, poseidon, indexedTransactions, provider, }) {
        const merkleTreeProgram = new anchor_1.Program(index_2.IDL_MERKLE_TREE_PROGRAM, index_1.merkleTreeProgramId, provider);
        let mtFetched = await merkleTreeProgram.account.transactionMerkleTree.fetch(pubkey, "processed");
        indexedTransactions.sort((a, b) => new anchor_1.BN(a.firstLeafIndex).toNumber() -
            new anchor_1.BN(b.firstLeafIndex).toNumber());
        const merkleTreeIndex = mtFetched.nextIndex;
        const leaves = [];
        if (indexedTransactions.length > 0) {
            for (let i = 0; i < indexedTransactions.length; i++) {
                if (new anchor_1.BN(indexedTransactions[i].firstLeafIndex).toNumber() <
                    merkleTreeIndex.toNumber()) {
                    for (const iterator of indexedTransactions[i].leaves) {
                        leaves.push(new anchor.BN(iterator, undefined, "le").toString());
                    }
                }
            }
        }
        let fetchedMerkleTree = new merkleTree_1.MerkleTree(index_1.MERKLE_TREE_HEIGHT, poseidon, leaves);
        // @ts-ignore: unknown type error
        let index = mtFetched.roots.findIndex((root) => {
            return (Array.from(leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32)).toString() === root.toString());
        });
        let retries = 3;
        while (index < 0 && retries > 0) {
            await (0, index_1.sleep)(100);
            retries--;
            mtFetched = await merkleTreeProgram.account.transactionMerkleTree.fetch(pubkey, "processed");
            // @ts-ignore: unknown type error
            index = mtFetched.roots.findIndex((root) => {
                return (Array.from(leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32)).toString() === root.toString());
            });
        }
        if (index < 0) {
            throw new Error(`building merkle tree from chain failed: root local ${Array.from(leInt2Buff(unstringifyBigInts(fetchedMerkleTree.root()), 32)).toString()} is not present in roots fetched`);
        }
        return new SolMerkleTree({ merkleTree: fetchedMerkleTree, pubkey });
    }
    static async getUninsertedLeaves(merkleTreePubkey, provider) {
        const { leavesAccounts, merkleTreeIndex } = await SolMerkleTree.getLeaves(merkleTreePubkey, provider);
        let filteredLeaves = leavesAccounts
            .filter((pda) => {
            if (pda.account.merkleTreePubkey.toBase58() ===
                merkleTreePubkey.toBase58()) {
                return (pda.account.leftLeafIndex.toNumber() >= merkleTreeIndex.toNumber());
            }
        })
            .sort((a, b) => a.account.leftLeafIndex.toNumber() -
            b.account.leftLeafIndex.toNumber());
        return filteredLeaves;
    }
    static async getUninsertedLeavesRelayer(merkleTreePubkey, provider) {
        return (await SolMerkleTree.getUninsertedLeaves(merkleTreePubkey, provider)).map((pda) => {
            return { isSigner: false, isWritable: true, pubkey: pda.publicKey };
        });
    }
}
exports.SolMerkleTree = SolMerkleTree;
//# sourceMappingURL=solMerkleTree.js.map