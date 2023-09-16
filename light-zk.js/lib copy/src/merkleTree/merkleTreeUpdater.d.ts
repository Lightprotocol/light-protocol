import { MerkleTreeProgram } from "../index";
import { PublicKey, Keypair, Connection } from "@solana/web3.js";
import { Program } from "@coral-xyz/anchor";
export declare function executeUpdateMerkleTreeTransactions({ signer, merkleTreeProgram, leavesPdas, transactionMerkleTree, connection, }: {
    signer: Keypair;
    merkleTreeProgram: Program<MerkleTreeProgram>;
    leavesPdas: any;
    transactionMerkleTree: PublicKey;
    connection: Connection;
}): Promise<void>;
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
export declare function executeMerkleTreeUpdateTransactions({ merkleTreeProgram, merkleTreeUpdateState, transactionMerkleTree, signer, connection, numberOfTransactions, interrupt, }: {
    numberOfTransactions?: number;
    merkleTreeProgram: Program<MerkleTreeProgram>;
    merkleTreeUpdateState: PublicKey;
    transactionMerkleTree: PublicKey;
    signer: Keypair;
    connection: Connection;
    interrupt?: boolean;
}): Promise<void>;
