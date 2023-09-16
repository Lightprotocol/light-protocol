/// <reference types="bn.js" />
import { PublicKey, TransactionSignature, TransactionInstruction } from "@solana/web3.js";
import { BN, Program } from "@coral-xyz/anchor";
import { Utxo } from "../utxo";
import { Provider, TransactionParameters, RelayerSendTransactionsResponse, SendVersionedTransactionsResult } from "../index";
import { IDL_MERKLE_TREE_PROGRAM } from "../idls/index";
import { remainingAccount } from "../types/accounts";
export declare enum Action {
    SHIELD = "SHIELD",
    TRANSFER = "TRANSFER",
    UNSHIELD = "UNSHIELD"
}
type PublicInputs = {
    root: Array<number>;
    publicAmountSpl: Array<number>;
    txIntegrityHash: Array<number>;
    publicAmountSol: Array<number>;
    publicMintPubkey: Array<number>;
    inputNullifier: Array<Array<number>>;
    outputCommitment: Array<Array<number>>;
    transactionHash?: Array<number>;
    checkedParams?: Array<Array<number>>;
    publicAppVerifier?: Array<number>;
};
export declare class Transaction {
    merkleTreeProgram?: Program<typeof IDL_MERKLE_TREE_PROGRAM>;
    shuffleEnabled: Boolean;
    params: TransactionParameters;
    appParams?: any;
    provider: Provider;
    transactionInputs: {
        publicInputs?: PublicInputs;
        rootIndex?: BN;
        proofBytes?: any;
        proofBytesApp?: any;
        publicInputsApp?: any;
        encryptedUtxos?: Uint8Array;
    };
    remainingAccounts?: {
        nullifierPdaPubkeys?: remainingAccount[];
        leavesPdaPubkeys?: remainingAccount[];
        nextTransactionMerkleTree?: remainingAccount;
    };
    proofInput: any;
    firstPath: string;
    /**
     * Initialize transaction
     *
     * @param relayer recipient of the unshielding
     * @param shuffleEnabled
     */
    constructor({ provider, shuffleEnabled, params, appParams, }: {
        provider: Provider;
        shuffleEnabled?: boolean;
        params: TransactionParameters;
        appParams?: any;
    });
    compileAndProve(): Promise<void>;
    /**
     * @description Prepares proof inputs.
     */
    compile(): Promise<void>;
    getMint(): string | BN;
    getProof(): Promise<void>;
    getAppProof(): Promise<void>;
    getProofInternal(params: TransactionParameters | any, firstPath: string): Promise<{
        parsedProof: {
            proofA: number[];
            proofB: number[][];
            proofC: number[];
        };
        parsedPublicInputsObject: {};
    }>;
    static getTransactionHash(params: TransactionParameters, poseidon: any): string;
    /**
     * @description fetches the merkle tree pda from the chain and checks in which index the root of the local merkle tree is.
     */
    getRootIndex(): Promise<void>;
    /**
     * @description Computes the indices in which the asset for the utxo is in the asset pubkeys array.
     * @note Using the indices the zero knowledege proof circuit enforces that only utxos containing the
     * @note assets in the asset pubkeys array are contained in the transaction.
     * @param utxos
     * @returns
     */
    getIndices(utxos: Utxo[]): string[][][];
    /**
     * @description Gets the merkle proofs for every input utxo with amounts > 0.
     * @description For input utxos with amounts == 0 it returns merkle paths with all elements = 0.
     */
    static getMerkleProofs(provider: Provider, inputUtxos: Utxo[]): {
        inputMerklePathIndices: Array<string>;
        inputMerklePathElements: Array<Array<string>>;
    };
    static getSignerAuthorityPda(merkleTreeProgramId: PublicKey, verifierProgramId: PublicKey): PublicKey;
    static getRegisteredVerifierPda(merkleTreeProgramId: PublicKey, verifierProgramId: PublicKey): PublicKey;
    sendAndConfirmTransaction(): Promise<RelayerSendTransactionsResponse | SendVersionedTransactionsResult>;
    /**
     * Asynchronously generates an array of transaction instructions based on the provided transaction parameters.
     *
     * 1. Validates that the required properties of transactionInputs and verifier are defined.
     * 2. Retrieves ordered instruction names from the verifier program by:
     *    a. Filtering instructions based on a suffix pattern (e.g., "First", "Second", "Third", etc.).
     *    b. Sorting instructions according to the order of suffixes.
     * 3. Constructs an input object containing the necessary data for encoding.
     * 4. Iterates through the instruction names, encoding the inputs and generating transaction instructions.
     * 5. Returns an array of generated transaction instructions.
     *
     * @param {TransactionParameters} params - Object containing the required transaction parameters.
     * @returns {Promise<TransactionInstruction[]>} - Promise resolving to an array of generated transaction instructions.
     */
    getInstructions(params: TransactionParameters): Promise<TransactionInstruction[]>;
    closeVerifierState(): Promise<TransactionSignature>;
    getPdaAddresses(): void;
    static getNullifierPdaPublicKey(nullifier: number[], merkleTreeProgramId: PublicKey): PublicKey;
    shuffleUtxos(utxos: Utxo[]): Utxo[] | undefined;
    static getTokenAuthority(): PublicKey;
}
export {};
