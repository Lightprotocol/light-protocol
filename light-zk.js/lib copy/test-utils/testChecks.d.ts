/// <reference types="node" />
import * as anchor from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import { MerkleTreeProgram } from "../idls";
import { Program } from "@coral-xyz/anchor";
export declare function checkMerkleTreeUpdateStateCreated({ connection, merkleTreeUpdateState, relayer, transactionMerkleTree, leavesPdas, current_instruction_index, merkleTreeProgram, }: {
    connection: Connection;
    merkleTreeUpdateState: PublicKey;
    relayer: PublicKey;
    transactionMerkleTree: PublicKey;
    leavesPdas: Array<any>;
    current_instruction_index: number;
    merkleTreeProgram: anchor.Program<MerkleTreeProgram>;
}): Promise<void>;
export declare function checkMerkleTreeBatchUpdateSuccess({ connection, merkleTreeUpdateState, merkleTreeAccountPrior, numberOfLeaves, transactionMerkleTree, merkleTreeProgram, }: {
    connection: Connection;
    merkleTreeUpdateState: PublicKey;
    merkleTreeAccountPrior: any;
    numberOfLeaves: number;
    leavesPdas: any;
    transactionMerkleTree: PublicKey;
    merkleTreeProgram: Program<MerkleTreeProgram>;
}): Promise<void>;
export declare function checkRentExemption({ connection, account, }: {
    connection: Connection;
    account: any;
}): Promise<void>;
export declare function checkNfInserted(pubkeys: {
    isSigner: boolean;
    isWritatble: boolean;
    pubkey: PublicKey;
}[], connection: Connection, returnValue?: boolean): Promise<anchor.web3.AccountInfo<Buffer> | null | undefined>;
