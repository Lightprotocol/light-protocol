import { Program, AnchorProvider, setProvider, BN } from '@coral-xyz/anchor';
import {
    PublicKey,
    Keypair,
    Connection,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import { Buffer } from 'buffer';

import {
    IDL,
    LightSystemProgram as LightSystemProgramIDL,
} from '../idls/light_system_program';
import { useWallet } from '../wallet';
import {
    CompressedAccountWithMerkleContext,
    CompressedProof,
    InstructionDataInvoke,
    OutputCompressedAccountWithPackedContext,
    bn,
    createOutputCompressedAccount,
} from '../state';
import { packCompressedAccounts } from '../instruction';
import { defaultStaticAccountsStruct } from '../constants';
import {
    validateSameOwner,
    validateSufficientBalance,
} from '../utils/validation';

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

    private static _program: Program<LightSystemProgramIDL> | null = null;

    static get program(): Program<LightSystemProgramIDL> {
        if (!this._program) {
            this.initializeProgram();
        }
        return this._program!;
    }

    /**
     * @internal
     * Cwct1kQLwJm8Z3HetLu8m4SXkhD6FZ5fXbJQCxTxPnGY
     *
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
        outputStateMerkleTreeIndices: number[],
    ): OutputCompressedAccountWithPackedContext[] {
        lamports = bn(lamports);
        const inputLamports = sumUpLamports(inputCompressedAccounts);
        const changeLamports = inputLamports.sub(lamports);

        validateSufficientBalance(changeLamports);

        if (changeLamports.eq(bn(0))) {
            return [
                createOutputCompressedAccount(
                    toAddress,
                    outputStateMerkleTreeIndices[0],
                    lamports,
                ),
            ];
        }

        validateSameOwner(inputCompressedAccounts);

        const outputCompressedAccounts: OutputCompressedAccountWithPackedContext[] =
            [
                createOutputCompressedAccount(
                    inputCompressedAccounts[0].owner,
                    outputStateMerkleTreeIndices[1],
                    changeLamports,
                ),
                createOutputCompressedAccount(
                    toAddress,
                    outputStateMerkleTreeIndices[2],
                    lamports,
                ),
            ];
        return outputCompressedAccounts;
    }

    static createDecompressOutputState(
        inputCompressedAccounts: CompressedAccountWithMerkleContext[],
        lamports: number | BN,
        outputMerkleTreeIndices: number[],
    ): OutputCompressedAccountWithPackedContext[] {
        lamports = bn(lamports);
        const inputLamports = sumUpLamports(inputCompressedAccounts);
        const changeLamports = inputLamports.sub(lamports);

        validateSufficientBalance(changeLamports);

        /// lamports gets decompressed
        if (changeLamports.eq(bn(0))) {
            return [];
        }

        validateSameOwner(inputCompressedAccounts);

        const outputCompressedAccounts: OutputCompressedAccountWithPackedContext[] =
            [
                createOutputCompressedAccount(
                    inputCompressedAccounts[0].owner,
                    outputMerkleTreeIndices[0],
                    changeLamports,
                ),
            ];
        return outputCompressedAccounts;
    }

    /**
     * Creates a transaction instruction that transfers compressed lamports from
     * one owner to another.
     */
    static async transfer({
        payer,
        inputCompressedAccounts,
        toAddress,
        lamports,
        recentInputStateRootIndices,
        recentValidityProof,
        outputStateTrees,
    }: TransferParams): Promise<TransactionInstruction> {
        /// Pack accounts
        const {
            packedInputCompressedAccounts,
            outputStateMerkleTreeIndices,
            remainingAccountMetas,
        } = packCompressedAccounts(
            inputCompressedAccounts,
            recentInputStateRootIndices,
            1,
            outputStateTrees,
        );
        /// Create output state
        const outputCompressedAccounts = this.createTransferOutputState(
            inputCompressedAccounts,
            toAddress,
            lamports,
            outputStateMerkleTreeIndices,
        );
        /// Encode instruction data
        const data = this.program.coder.types.encode('InstructionDataInvoke', {
            proof: recentValidityProof,
            /// TODO: here and on-chain: option<newAddressInputs> or similar.
            newAddressParams: [],
            inputCompressedAccountsWithMerkleContext:
                packedInputCompressedAccounts,
            outputCompressedAccounts,
            relayFee: null,
            compressionLamports: null,
            isCompress: false,
            signerSeeds: null,
            cpiContext: null,
        });

        /// Build anchor instruction
        const instruction = await this.program.methods
            .invoke(data)
            .accounts({
                ...defaultStaticAccountsStruct(),
                feePayer: payer,
                authority: payer,
                compressedSolPda: null,
                compressionRecipient: null,
                systemProgram: SystemProgram.programId,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        return instruction;
    }

    /**
     * Creates a transaction instruction that transfers compressed lamports from
     * one owner to another.
     */
    // TODO: add support for non-fee-payer owner
    static async compress({
        payer,
        toAddress,
        lamports,
        outputStateTree,
    }: CompressParams): Promise<TransactionInstruction> {
        /// Create output state
        lamports = bn(lamports);

        /// Pack accounts
        const {
            packedInputCompressedAccounts,
            outputStateMerkleTreeIndices,
            remainingAccountMetas,
        } = packCompressedAccounts([], [], 1, outputStateTree);
        const outputCompressedAccount = createOutputCompressedAccount(
            toAddress,
            outputStateMerkleTreeIndices[0],
            lamports,
        );
        /// Encode instruction data
        const rawInputs: InstructionDataInvoke = {
            proof: null,
            /// TODO: here and on-chain: option<newAddressInputs> or similar.
            newAddressParams: [],
            inputCompressedAccountsWithMerkleContext:
                packedInputCompressedAccounts,
            outputCompressedAccounts: [outputCompressedAccount],
            relayFee: null,
            compressionLamports: lamports,
            isCompress: true,
        };

        const data = this.program.coder.types.encode(
            'InstructionDataInvoke',
            rawInputs,
        );

        /// Build anchor instruction
        const instruction = await this.program.methods
            .invoke(data)
            .accounts({
                ...defaultStaticAccountsStruct(),
                feePayer: payer,
                authority: payer,
                compressedSolPda: this.deriveCompressedSolPda(),
                compressionRecipient: null,
                systemProgram: SystemProgram.programId,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        return instruction;
    }

    /**
     * Creates a transaction instruction that transfers compressed lamports from
     * one owner to another.
     */
    /// TODO: add check that outputStateTree is provided or supplemented if change exists
    static async decompress({
        payer,
        inputCompressedAccounts,
        toAddress,
        lamports,
        recentInputStateRootIndices,
        recentValidityProof,
        outputStateTree,
    }: DecompressParams): Promise<TransactionInstruction> {
        /// Create output state
        lamports = bn(lamports);

        /// Pack accounts
        const {
            packedInputCompressedAccounts,
            outputStateMerkleTreeIndices,
            remainingAccountMetas,
        } = packCompressedAccounts(
            inputCompressedAccounts,
            recentInputStateRootIndices,
            1,
            outputStateTree,
        );
        const outputCompressedAccounts = this.createDecompressOutputState(
            inputCompressedAccounts,
            lamports,
            outputStateMerkleTreeIndices,
        );
        /// Encode instruction data
        const data = this.program.coder.types.encode('InstructionDataInvoke', {
            proof: recentValidityProof,
            /// TODO: here and on-chain: option<newAddressInputs> or similar.
            newAddressParams: [],
            inputCompressedAccountsWithMerkleContext:
                packedInputCompressedAccounts,
            outputCompressedAccounts: outputCompressedAccounts,
            relayFee: null,
            compressionLamports: lamports,
            isCompress: false,
            signerSeeds: null,
            cpiContext: null,
        });

        /// Build anchor instruction
        const instruction = await this.program.methods
            .invoke(data)
            .accounts({
                ...defaultStaticAccountsStruct(),
                feePayer: payer,
                authority: payer,
                compressedSolPda: this.deriveCompressedSolPda(),
                compressionRecipient: toAddress,
                systemProgram: SystemProgram.programId,
            })
            .remainingAccounts(remainingAccountMetas)
            .instruction();

        return instruction;
    }
}

/**
 * Selects the minimal number of compressed SOL accounts for a transfer.
 *
 * 1. Sorts the accounts by amount in descending order
 * 2. Accumulates the amount until it is greater than or equal to the transfer
 *    amount
 */
export function selectMinCompressedSolAccountsForTransfer(
    accounts: CompressedAccountWithMerkleContext[],
    transferLamports: BN | number,
): [selectedAccounts: CompressedAccountWithMerkleContext[], total: BN] {
    let accumulatedLamports = bn(0);
    transferLamports = bn(transferLamports);

    const selectedAccounts: CompressedAccountWithMerkleContext[] = [];

    accounts.sort((a, b) => b.lamports.cmp(a.lamports));

    for (const account of accounts) {
        if (accumulatedLamports.gte(bn(transferLamports))) break;
        accumulatedLamports = accumulatedLamports.add(account.lamports);
        selectedAccounts.push(account);
    }

    if (accumulatedLamports.lt(bn(transferLamports))) {
        throw new Error(
            `Not enough balance for transfer. Required: ${transferLamports.toString()}, available: ${accumulatedLamports.toString()}`,
        );
    }

    return [selectedAccounts, accumulatedLamports];
}
