/// <reference types="bn.js" />
import { BN, Provider } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { ParsedIndexedTransaction } from "../index";
import { MerkleTree } from "./merkleTree";
export type QueuedLeavesPda = {
    leftLeafIndex: BN;
    encryptedUtxos: Array<number>;
    nodeLeft: Array<number>;
    nodeRight: Array<number>;
    merkleTreePubkey: PublicKey;
};
export declare class SolMerkleTree {
    merkleTree: MerkleTree;
    pubkey: PublicKey;
    constructor({ pubkey, poseidon, merkleTree, }: {
        poseidon?: any;
        merkleTree?: MerkleTree;
        pubkey: PublicKey;
    });
    static getLeaves(merkleTreePubkey: PublicKey, provider?: Provider): Promise<{
        leavesAccounts: any[];
        merkleTreeIndex: BN;
        mtFetched: {
            filledSubtrees: number[][];
            currentRootIndex: BN;
            nextIndex: BN;
            roots: number[][];
            pubkeyLocked: PublicKey;
            timeLocked: BN;
            height: BN;
            merkleTreeNr: BN;
            lockDuration: BN;
            nextQueuedIndex: BN;
            newest: number;
            padding: number[];
        };
    }>;
    static build({ pubkey, poseidon, indexedTransactions, provider, }: {
        pubkey: PublicKey;
        poseidon: any;
        indexedTransactions: ParsedIndexedTransaction[];
        provider?: Provider;
    }): Promise<SolMerkleTree>;
    static getUninsertedLeaves(merkleTreePubkey: PublicKey, provider?: Provider): Promise<Array<{
        publicKey: PublicKey;
        account: any;
    }>>;
    static getUninsertedLeavesRelayer(merkleTreePubkey: PublicKey, provider?: Provider): Promise<{
        isSigner: boolean;
        isWritable: boolean;
        pubkey: PublicKey;
    }[]>;
}
//# sourceMappingURL=solMerkleTree.d.ts.map