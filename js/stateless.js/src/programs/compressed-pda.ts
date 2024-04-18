import { Program, AnchorProvider, setProvider, BN } from '@coral-xyz/anchor';
import {
    PublicKey,
    Keypair,
    Connection,
    TransactionInstruction,
    AccountMeta,
    ComputeBudgetProgram,
    SystemProgram,
} from '@solana/web3.js';
import { IDL, LightCompressedPda } from '../idls/light_compressed_pda';
import { useWallet } from '../wallet';
import {
    CompressedAccount,
    CompressedAccountWithMerkleContext,
    CompressedProof,
    bn,
    createCompressedAccount,
} from '../state';
import { packCompressedAccounts } from '../instruction';
import { defaultStaticAccountsStruct } from '../constants';
import {
    validateSameOwner,
    validateSufficientBalance,
} from '../utils/validation';
import { placeholderValidityProof } from '../test-utils';

export const sumUpLamports = (
    accounts: CompressedAccountWithMerkleContext[],
): BN => {
    return accounts.reduce(
        (acc, account) => acc.add(bn(account.lamports)),
        bn(0),
    );
};

/**
 * Defines the parameters for the transfer method
 */
type TransferParams = {
    /**
     * The payer of the transaction.
     */
    payer: PublicKey;
    /**
     * The input state to be consumed.
     */
    inputCompressedAccounts: CompressedAccountWithMerkleContext[];
    /**
     * Recipient address
     */
    toAddress: PublicKey;
    /**
     * amount of lamports to transfer.
     */
    lamports: number | BN;
    /**
     * The recent state root indices of the input state. The expiry is tied to
     * the proof.
     *
     * TODO: Add support for passing recent-values after instruction creation.
     */
    recentInputStateRootIndices: number[];
    /**
     * The recent validity proof for state inclusion of the input state. It
     * expires after n slots.
     */
    recentValidityProof: CompressedProof;
    /**
     * The state trees that the tx output should be inserted into. This can be a
     * single PublicKey or an array of PublicKey. Defaults to the 0th state tree
     * of input state.
     */
    outputStateTrees?: PublicKey[] | PublicKey;
};

/// TODO:
/// - add option to compress to another owner
/// - add option to merge with input state
/**
 * Defines the parameters for the transfer method
 */
type CompressParams = {
    /**
     * The payer of the transaction.
     */
    payer: PublicKey;
    /**
     * address that the lamports are attached to. also defaults to the recipient owner
     */
    toAddress: PublicKey;
    /**
     * amount of lamports to compress.
     */
    lamports: number | BN;
    /**
     * The state tree that the tx output should be inserted into. This can be a
     *
     */
    outputStateTree: PublicKey;
};

/**
 * Defines the parameters for the transfer method
 */
type DecompressParams = {
    /**
     * The payer of the transaction.
     */
    payer: PublicKey;
    /**
     * The input state to be consumed.
     */
    inputCompressedAccounts: CompressedAccountWithMerkleContext[];
    /**
     * Recipient address of uncompressed lamports
     */
    toAddress: PublicKey;
    /**
     * amount of lamports to decompress.
     */
    lamports: number | BN;
    /**
     * The recent state root indices of the input state. The expiry is tied to
     * the proof.
     *
     * TODO: Add support for passing recent-values after instruction creation.
     */
    recentInputStateRootIndices: number[];
    /**
     * The recent validity proof for state inclusion of the input state. It
     * expires after n slots.
     */
    recentValidityProof: CompressedProof;
    /**
     * The state trees that the tx output should be inserted into. This can be a
     * single PublicKey or an array of PublicKey. Defaults to the 0th state tree
     * of input state.
     */
    outputStateTree?: PublicKey;
};

const COMPRESSED_SOL_PDA_SEED = Buffer.from('compressed_sol_pda');

export class LightSystemProgram {
    /**
     * @internal
     */
    constructor() {}

    /**
     * Public key that identifies the CompressedPda program
     */
    static programId: PublicKey = new PublicKey(
        // TODO: can add check to ensure its consistent with the idl
        '6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ',
    );

    private static _program: Program<LightCompressedPda> | null = null;

    static get program(): Program<LightCompressedPda> {
        if (!this._program) {
            this.initializeProgram();
        }
        return this._program!;
    }

    /**
     * @internal
     * Cwct1kQLwJm8Z3HetLu8m4SXkhD6FZ5fXbJQCxTxPnGY
     */
    static deriveCompressedSolPda(): PublicKey {
        const seeds = [COMPRESSED_SOL_PDA_SEED];
        const [address, _] = PublicKey.findProgramAddressSync(
            seeds,
            this.programId,
        );
        return address;
    }
    /**
     * Initializes the program statically if not already initialized.
     */
    private static initializeProgram() {
        if (!this._program) {
            const mockKeypair = Keypair.generate();
            const mockConnection = new Connection(
                'http://127.0.0.1:8899',
                'confirmed',
            );
            const mockProvider = new AnchorProvider(
                mockConnection,
                useWallet(mockKeypair),
                {
                    commitment: 'confirmed',
                    preflightCommitment: 'confirmed',
                },
            );
            setProvider(mockProvider);
            this._program = new Program(IDL, this.programId, mockProvider);
        }
    }

    static createTransferOutputState(
        inputCompressedAccounts: CompressedAccountWithMerkleContext[],
        toAddress: PublicKey,
        lamports: number | BN,
    ): CompressedAccount[] {
        lamports = bn(lamports);
        const inputLamports = sumUpLamports(inputCompressedAccounts);
        const changeLamports = inputLamports.sub(lamports);

        validateSufficientBalance(changeLamports);

        if (changeLamports.eq(bn(0))) {
            return [createCompressedAccount(toAddress, lamports)];
        }

        validateSameOwner(inputCompressedAccounts);

        const outputCompressedAccounts: CompressedAccount[] = [
            createCompressedAccount(toAddress, lamports),
            createCompressedAccount(
                inputCompressedAccounts[0].owner,
                changeLamports,
            ),
        ];
        return outputCompressedAccounts;
    }

    static createDecompressOutputState(
        inputCompressedAccounts: CompressedAccountWithMerkleContext[],
        lamports: number | BN,
    ): CompressedAccount[] {
        lamports = bn(lamports);
        const inputLamports = sumUpLamports(inputCompressedAccounts);
        const changeLamports = inputLamports.sub(lamports);

        validateSufficientBalance(changeLamports);

        /// lamports gets decompressed
        if (changeLamports.eq(bn(0))) {
            return [];
        }

        validateSameOwner(inputCompressedAccounts);

        const outputCompressedAccounts: CompressedAccount[] = [
            createCompressedAccount(
                inputCompressedAccounts[0].owner,
                changeLamports,
            ),
        ];
        return outputCompressedAccounts;
    }

    /**
     * Creates a transaction instruction that transfers compressed lamports from
     * one owner to another.
     */
    static async transfer(
        params: TransferParams,
    ): Promise<TransactionInstruction[]> {
        const {
            payer,
            recentValidityProof,
            recentInputStateRootIndices,
            inputCompressedAccounts,
            lamports,
            outputStateTrees,
        } = params;

        /// Create output state
        const outputCompressedAccounts = this.createTransferOutputState(
            inputCompressedAccounts,
            params.toAddress,
            lamports,
        );

        /// Pack accounts
        const {
            packedInputCompressedAccounts,
            outputStateMerkleTreeIndices,
            remainingAccountMetas,
        } = packCompressedAccounts(
            inputCompressedAccounts,
            outputCompressedAccounts.length,
            outputStateTrees,
        );

        /// Encode instruction data
        const data = this.program.coder.types.encode(
            'InstructionDataTransfer',
            {
                proof: recentValidityProof,
                inputRootIndices: recentInputStateRootIndices,
                /// TODO: here and on-chain: option<newAddressInputs> or similar.
                newAddressParams: [],
                inputCompressedAccountsWithMerkleContext:
                    packedInputCompressedAccounts,
                outputCompressedAccounts,
                outputStateMerkleTreeAccountIndices: Buffer.from(
                    outputStateMerkleTreeIndices,
                ),
                relayFee: null,
                compressionLamports: null,
                isCompress: false,
            },
        );

        /// Build anchor instruction
        const instruction = await this.program.methods
            .executeCompressedTransaction(data)
            .accounts({
                ...defaultStaticAccountsStruct(),
                signer: payer,
                invokingProgram: this.programId,
                compressedSolPda: null,
                compressionRecipient: null,
                systemProgram: null,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        const instructions = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
            instruction,
        ];

        return instructions;
    }

    /**
     * Initialize the compressed sol pda
     */
    static async initCompressedSolPda(
        feePayer: PublicKey,
    ): Promise<TransactionInstruction> {
        const accounts = {
            feePayer,
            compressedSolPda: this.deriveCompressedSolPda(),
            systemProgram: SystemProgram.programId,
        };

        const instruction = await this.program.methods
            .initCompressSolPda()
            .accounts(accounts)
            .instruction();
        return instruction;
    }

    /**
     * Creates a transaction instruction that transfers compressed lamports from
     * one owner to another.
     */
    // TODO: add support for non-fee-payer owner
    static async compress(
        params: CompressParams,
    ): Promise<TransactionInstruction[]> {
        const { payer, outputStateTree, toAddress } = params;

        /// Create output state
        const lamports = bn(params.lamports);
        const outputCompressedAccount = createCompressedAccount(
            toAddress,
            lamports,
        );

        /// Pack accounts
        const {
            packedInputCompressedAccounts,
            outputStateMerkleTreeIndices,
            remainingAccountMetas,
        } = packCompressedAccounts([], 1, outputStateTree);

        /// Encode instruction data
        const data = this.program.coder.types.encode(
            'InstructionDataTransfer',
            {
                proof: placeholderValidityProof(),
                inputRootIndices: [],
                /// TODO: here and on-chain: option<newAddressInputs> or similar.
                newAddressParams: [],
                inputCompressedAccountsWithMerkleContext:
                    packedInputCompressedAccounts,
                outputCompressedAccounts: [outputCompressedAccount],
                outputStateMerkleTreeAccountIndices: Buffer.from(
                    new Uint8Array(outputStateMerkleTreeIndices),
                ),
                relayFee: null,
                compressionLamports: lamports,
                isCompress: true,
            },
        );

        /// Build anchor instruction
        const instruction = await this.program.methods
            .executeCompressedTransaction(data)
            .accounts({
                ...defaultStaticAccountsStruct(),
                signer: payer,
                invokingProgram: this.programId,
                compressedSolPda: this.deriveCompressedSolPda(),
                compressionRecipient: null,
                systemProgram: SystemProgram.programId,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        const instructions = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
            instruction,
        ];

        return instructions;
    }

    /**
     * Creates a transaction instruction that transfers compressed lamports from
     * one owner to another.
     */
    /// TODO: add check that outputStateTree is provided or supplemented if change exists
    static async decompress(
        params: DecompressParams,
    ): Promise<TransactionInstruction[]> {
        const { payer, outputStateTree, toAddress } = params;

        /// Create output state
        const lamports = bn(params.lamports);
        const outputCompressedAccounts = this.createDecompressOutputState(
            params.inputCompressedAccounts,
            lamports,
        );

        /// Pack accounts
        const {
            packedInputCompressedAccounts,
            outputStateMerkleTreeIndices,
            remainingAccountMetas,
        } = packCompressedAccounts(
            params.inputCompressedAccounts,
            outputCompressedAccounts.length,
            outputStateTree,
        );

        /// Encode instruction data
        const data = this.program.coder.types.encode(
            'InstructionDataTransfer',
            {
                proof: params.recentValidityProof,
                inputRootIndices: params.recentInputStateRootIndices,
                /// TODO: here and on-chain: option<newAddressInputs> or similar.
                newAddressParams: [],
                inputCompressedAccountsWithMerkleContext:
                    packedInputCompressedAccounts,
                outputCompressedAccounts: outputCompressedAccounts,
                outputStateMerkleTreeAccountIndices: Buffer.from(
                    new Uint8Array(outputStateMerkleTreeIndices),
                ),
                relayFee: null,
                compressionLamports: lamports,
                isCompress: false,
            },
        );

        /// Build anchor instruction
        const instruction = await this.program.methods
            .executeCompressedTransaction(data)
            .accounts({
                ...defaultStaticAccountsStruct(),
                signer: payer,
                invokingProgram: this.programId,
                compressedSolPda: this.deriveCompressedSolPda(),
                compressionRecipient: toAddress,
                systemProgram: SystemProgram.programId,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        const instructions = [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
            instruction,
        ];

        return instructions;
    }
}

// /**
//  * @internal
//  *
//  * Selects the minimal number of compressed accounts for a transfer
//  * 1. Sorts the accounts by amount in descending order
//  * 2. Accumulates the lamports amount until it is greater than or equal to the transfer
//  *    amount
//  */
// function _selectMinCompressedAccountsForTransfer(
//     compressedAccounts: (UtxoWithMerkleContext | UtxoWithMerkleProof)[],
//     transferAmount: BN,
// ): {
//     selectedAccounts: (UtxoWithMerkleContext | UtxoWithMerkleProof)[];
//     total: BN;
// } {
//     let accumulatedAmount = bn(0);
//     const selectedAccounts: (UtxoWithMerkleContext | UtxoWithMerkleProof)[] =
//         [];

//     compressedAccounts.sort((a, b) =>
//         Number(bn(b.lamports).sub(bn(a.lamports))),
//     );
//     for (const utxo of compressedAccounts) {
//         if (accumulatedAmount.gte(bn(transferAmount))) break;
//         accumulatedAmount = accumulatedAmount.add(bn(utxo.lamports));
//         selectedAccounts.push(utxo);
//     }
//     if (accumulatedAmount.lt(bn(transferAmount))) {
//         throw new Error('Not enough balance for transfer');
//     }
//     return { selectedAccounts, total: accumulatedAmount };
// }
