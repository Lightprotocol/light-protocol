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
    CompressedAccount,
    CompressedAccountWithMerkleContext,
    CompressedProof,
    InstructionDataInvoke,
    bn,
    createCompressedAccount,
} from '../state';
import { packCompressedAccounts, toAccountMetas } from '../instruction';
import {
    defaultStaticAccountsStruct,
    defaultTestStateTreeAccounts,
} from '../constants';
import {
    validateSameOwner,
    validateSufficientBalance,
} from '../utils/validation';
import { packNewAddressParams, NewAddressParams } from '../utils';

export const sumUpLamports = (
    accounts: CompressedAccountWithMerkleContext[],
): BN => {
    return accounts.reduce(
        (acc, account) => acc.add(bn(account.lamports)),
        bn(0),
    );
};

/**
 * Create compressed account system transaction params
 */
type CreateAccountWithSeedParams = {
    /**
     * The payer of the transaction.
     */
    payer: PublicKey;
    /**
     * Address params for the new compressed account
     */
    newAddressParams: NewAddressParams;
    newAddress: number[];
    /**
     * Recent validity proof proving that there's no existing compressed account
     * registered with newAccountAddress
     */
    recentValidityProof: CompressedProof;
    /**
     * State tree pubkey. Defaults to a public state tree if unspecified.
     */
    outputStateTree?: PublicKey;
    /**
     * Public key of the program to assign as the owner of the created account
     */
    programId?: PublicKey;
    /**
     * Optional input accounts to transfer lamports from into the new compressed
     * account.
     */
    inputCompressedAccounts?: CompressedAccountWithMerkleContext[];
    /**
     * Optional input state root indices of 'inputCompressedAccounts'. The
     * expiry is tied to the 'recentValidityProof'.
     */
    inputStateRootIndices?: number[];
    /**
     * Optional lamports to transfer into the new compressed account.
     */
    lamports?: number | BN;
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
     * The state tree that the tx output should be inserted into. Defaults to a
     * public state tree if unspecified.
     */
    outputStateTree?: PublicKey;
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

const SOL_POOL_PDA_SEED = Buffer.from('sol_pool_pda');

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
        'H5sFv8VwWmjxHYS2GB4fTDsK7uTtnRT4WiixtHrET3bN',
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
        const seeds = [SOL_POOL_PDA_SEED];
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
            createCompressedAccount(
                inputCompressedAccounts[0].owner,

                changeLamports,
            ),
            createCompressedAccount(toAddress, lamports),
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
     * No data by default
     */
    static createNewAddressOutputState(
        address: number[],
        owner: PublicKey,
        lamports?: BN | number,
        inputCompressedAccounts?: CompressedAccountWithMerkleContext[],
    ): CompressedAccount[] {
        lamports = bn(lamports ?? 0);
        const inputLamports = sumUpLamports(inputCompressedAccounts ?? []);
        const changeLamports = inputLamports.sub(lamports);

        validateSufficientBalance(changeLamports);

        if (changeLamports.eq(bn(0)) || !inputCompressedAccounts) {
            return [
                createCompressedAccount(owner, lamports, undefined, address),
            ];
        }

        validateSameOwner(inputCompressedAccounts);
        const outputCompressedAccounts: CompressedAccount[] = [
            createCompressedAccount(
                inputCompressedAccounts[0].owner,
                changeLamports,
            ),
            createCompressedAccount(owner, lamports, undefined, address),
        ];
        return outputCompressedAccounts;
    }

    /**
     * Creates instruction to create compressed account with PDA.
     * Cannot write data.
     *
     * TODO: support transfer of lamports to the new account.
     */
    static async createAccount({
        payer,
        newAddressParams,
        newAddress,
        recentValidityProof,
        outputStateTree,
        inputCompressedAccounts,
        inputStateRootIndices,
        lamports,
    }: CreateAccountWithSeedParams): Promise<TransactionInstruction> {
        const outputCompressedAccounts = this.createNewAddressOutputState(
            newAddress,
            payer,
            lamports,
            inputCompressedAccounts,
        );
        /// Pack accounts
        const {
            packedInputCompressedAccounts,
            packedOutputCompressedAccounts,
            remainingAccounts: _remainingAccounts,
        } = packCompressedAccounts(
            inputCompressedAccounts ?? [],
            inputStateRootIndices ?? [],
            outputCompressedAccounts,
            outputStateTree,
        );

        const { newAddressParamsPacked, remainingAccounts } =
            packNewAddressParams([newAddressParams], _remainingAccounts);

        const rawData: InstructionDataInvoke = {
            proof: recentValidityProof,
            inputCompressedAccountsWithMerkleContext:
                packedInputCompressedAccounts,
            outputCompressedAccounts: packedOutputCompressedAccounts,
            relayFee: null,
            newAddressParams: newAddressParamsPacked,
            compressOrDecompressLamports: null,
            isCompress: false,
        };

        /// Encode instruction data
        const ixData = this.program.coder.types.encode(
            'InstructionDataInvoke',
            rawData,
        );

        /// Build anchor instruction
        const instruction = await this.program.methods
            .invoke(ixData)
            .accounts({
                ...defaultStaticAccountsStruct(),
                feePayer: payer,
                authority: payer,
                solPoolPda: null,
                decompressionRecipient: null,
                systemProgram: SystemProgram.programId,
            })
            .remainingAccounts(toAccountMetas(remainingAccounts))
            .instruction();

        return instruction;
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
        /// Create output state
        const outputCompressedAccounts = this.createTransferOutputState(
            inputCompressedAccounts,
            toAddress,
            lamports,
        );
        /// Pack accounts
        const {
            packedInputCompressedAccounts,
            packedOutputCompressedAccounts,
            remainingAccounts,
        } = packCompressedAccounts(
            inputCompressedAccounts,
            recentInputStateRootIndices,
            outputCompressedAccounts,
            outputStateTrees,
        );
        /// Encode instruction data
        const data = this.program.coder.types.encode('InstructionDataInvoke', {
            proof: recentValidityProof,
            inputCompressedAccountsWithMerkleContext:
                packedInputCompressedAccounts,
            outputCompressedAccounts: packedOutputCompressedAccounts,
            relayFee: null,
            /// TODO: here and on-chain: option<newAddressInputs> or similar.
            newAddressParams: [],
            compressOrDecompressLamports: null,
            isCompress: false,
        });

        /// Build anchor instruction
        const instruction = await this.program.methods
            .invoke(data)
            .accounts({
                ...defaultStaticAccountsStruct(),
                feePayer: payer,
                authority: payer,
                solPoolPda: null,
                decompressionRecipient: null,
                systemProgram: SystemProgram.programId,
            })
            .remainingAccounts(toAccountMetas(remainingAccounts))
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

        const outputCompressedAccount = createCompressedAccount(
            toAddress,
            lamports,
        );

        /// Pack accounts
        const {
            packedInputCompressedAccounts,
            packedOutputCompressedAccounts,
            remainingAccounts,
        } = packCompressedAccounts(
            [],
            [],
            [outputCompressedAccount],
            outputStateTree,
        );

        /// Encode instruction data
        const rawInputs: InstructionDataInvoke = {
            proof: null,
            inputCompressedAccountsWithMerkleContext:
                packedInputCompressedAccounts,
            outputCompressedAccounts: packedOutputCompressedAccounts,
            relayFee: null,
            /// TODO: here and on-chain: option<newAddressInputs> or similar.
            newAddressParams: [],
            compressOrDecompressLamports: lamports,
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
                solPoolPda: this.deriveCompressedSolPda(),
                decompressionRecipient: null,
                systemProgram: SystemProgram.programId,
            })
            .remainingAccounts(toAccountMetas(remainingAccounts))
            .instruction();

        return instruction;
    }

    /**
     * Creates a transaction instruction that transfers compressed lamports from
     * one owner to another.
     */
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

        const outputCompressedAccounts = this.createDecompressOutputState(
            inputCompressedAccounts,
            lamports,
        );

        /// Pack accounts
        const {
            packedInputCompressedAccounts,
            packedOutputCompressedAccounts,
            remainingAccounts,
        } = packCompressedAccounts(
            inputCompressedAccounts,
            recentInputStateRootIndices,
            outputCompressedAccounts,
            outputStateTree,
        );
        /// Encode instruction data
        const data = this.program.coder.types.encode('InstructionDataInvoke', {
            proof: recentValidityProof,
            inputCompressedAccountsWithMerkleContext:
                packedInputCompressedAccounts,
            outputCompressedAccounts: packedOutputCompressedAccounts,
            relayFee: null,
            /// TODO: here and on-chain: option<newAddressInputs> or similar.
            newAddressParams: [],
            compressOrDecompressLamports: lamports,
            isCompress: false,
        });

        /// Build anchor instruction
        const instruction = await this.program.methods
            .invoke(data)
            .accounts({
                ...defaultStaticAccountsStruct(),
                feePayer: payer,
                authority: payer,
                solPoolPda: this.deriveCompressedSolPda(),
                decompressionRecipient: toAddress,
                systemProgram: SystemProgram.programId,
            })
            .remainingAccounts(toAccountMetas(remainingAccounts))
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
