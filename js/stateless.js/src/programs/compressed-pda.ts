import { Program, AnchorProvider, setProvider, BN } from '@coral-xyz/anchor';
import {
    PublicKey,
    Keypair,
    Connection,
    TransactionInstruction,
    AccountMeta,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import { IDL, PspCompressedPda } from '../idls/psp_compressed_pda';
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

const sumupLamports = (accounts: CompressedAccountWithMerkleContext[]): BN => {
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

    private static _program: Program<PspCompressedPda> | null = null;

    static get program(): Program<PspCompressedPda> {
        if (!this._program) {
            this.initializeProgram();
        }
        return this._program!;
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
        const inputLamports = sumupLamports(inputCompressedAccounts);
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
            remainingAccounts,
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
                newAddressSeeds: [],
                addressQueueAccountIndices: Buffer.from([]),
                addressMerkleTreeAccountIndices: Buffer.from([]),
                addressMerkleTreeRootIndices: [],
                inputCompressedAccountsWithMerkleContext:
                    packedInputCompressedAccounts,
                outputCompressedAccounts,
                outputStateMerkleTreeAccountIndices: Buffer.from(
                    outputStateMerkleTreeIndices,
                ),
                relayFee: null,
            },
        );

        /// Format accounts
        const staticAccounts = {
            ...defaultStaticAccountsStruct(),
            signer: payer,
            invokingProgram: this.programId,
        };

        const remainingAccountMetas = remainingAccounts.map(
            (account): AccountMeta => ({
                pubkey: account,
                isWritable: true,
                isSigner: false,
            }),
        );

        /// Build anchor instruction
        const instruction = await this.program.methods
            .executeCompressedTransaction(data)
            .accounts(staticAccounts)
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
