import BN from 'bn.js';
import {
    PublicKey,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    CompressedAccount,
    CompressedAccountWithMerkleContext,
    ValidityProof,
    InstructionDataInvoke,
    TreeInfo,
    bn,
    createCompressedAccount,
} from '../../state';
import {
    packCompressedAccounts,
    toAccountMetas,
} from '../../programs/system/pack';
import { defaultStaticAccountsStruct } from '../../constants';
import {
    validateSameOwner,
    validateSufficientBalance,
} from '../../utils/validation';
import { packNewAddressParams, NewAddressParams } from '../../utils';
import { encodeInstructionDataInvoke, invokeAccountsLayout } from './layout';

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
     * Address params for the new compressed account.
     */
    newAddressParams: NewAddressParams;
    /**
     * Address of the new compressed account
     */
    newAddress: number[];
    /**
     * Recent validity proof proving that there's no existing compressed account
     * registered with newAccountAddress
     */
    recentValidityProof: ValidityProof | null;
    /**
     * State tree pubkey. Defaults to a public state tree if unspecified.
     */
    outputStateTreeInfo?: TreeInfo;
    /**
     * Public key of the program to assign as the owner of the created account.
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
     * Recipient address.
     */
    toAddress: PublicKey;
    /**
     * Amount of lamports to transfer.
     */
    lamports: number | BN;
    /**
     * The recent state root indices of the input state. The expiry is tied to
     * the proof.
     */
    recentInputStateRootIndices: number[];
    /**
     * The recent validity proof for state inclusion of the input state. Expires
     * after n slots.
     */
    recentValidityProof: ValidityProof | null;
};

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
     * The state tree that the tx output should be inserted into.
     */
    outputStateTreeInfo: TreeInfo;
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
     */
    recentInputStateRootIndices: number[];
    /**
     * The recent validity proof for state inclusion of the input state. It
     * expires after n slots.
     */
    recentValidityProof: ValidityProof | null;
};

const SOL_POOL_PDA_SEED = Buffer.from('sol_pool_pda');

export class LightSystemProgram {
    /**
     * @internal
     */
    constructor() {}

    /**
     * The LightSystemProgram program ID.
     */
    static programId: PublicKey = new PublicKey(
        'SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7',
    );

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
     */
    static async createAccount({
        payer,
        newAddressParams,
        newAddress,
        recentValidityProof,
        outputStateTreeInfo,
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
            !inputCompressedAccounts || inputCompressedAccounts.length === 0
                ? outputStateTreeInfo
                : undefined,
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
        const data = encodeInstructionDataInvoke(rawData);

        const accounts = invokeAccountsLayout({
            ...defaultStaticAccountsStruct(),
            feePayer: payer,
            authority: payer,
            solPoolPda: null,
            decompressionRecipient: null,
            systemProgram: SystemProgram.programId,
        });
        const keys = [...accounts, ...toAccountMetas(remainingAccounts)];

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data,
        });
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
        );

        /// Encode instruction data
        const rawInputs: InstructionDataInvoke = {
            proof: recentValidityProof,
            inputCompressedAccountsWithMerkleContext:
                packedInputCompressedAccounts,
            outputCompressedAccounts: packedOutputCompressedAccounts,
            relayFee: null,
            newAddressParams: [],
            compressOrDecompressLamports: null,
            isCompress: false,
        };

        const data = encodeInstructionDataInvoke(rawInputs);

        const accounts = invokeAccountsLayout({
            ...defaultStaticAccountsStruct(),
            feePayer: payer,
            authority: payer,
            solPoolPda: null,
            decompressionRecipient: null,
            systemProgram: SystemProgram.programId,
        });

        const keys = [...accounts, ...toAccountMetas(remainingAccounts)];

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data,
        });
    }

    /**
     * Creates a transaction instruction that transfers compressed lamports from
     * one owner to another.
     */
    static async compress({
        payer,
        toAddress,
        lamports,
        outputStateTreeInfo,
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
            outputStateTreeInfo,
        );

        /// Encode instruction data
        const rawInputs: InstructionDataInvoke = {
            proof: null,
            inputCompressedAccountsWithMerkleContext:
                packedInputCompressedAccounts,
            outputCompressedAccounts: packedOutputCompressedAccounts,
            relayFee: null,
            newAddressParams: [],
            compressOrDecompressLamports: lamports,
            isCompress: true,
        };

        const data = encodeInstructionDataInvoke(rawInputs);

        const accounts = invokeAccountsLayout({
            ...defaultStaticAccountsStruct(),
            feePayer: payer,
            authority: payer,
            solPoolPda: LightSystemProgram.deriveCompressedSolPda(),
            decompressionRecipient: null,
            systemProgram: SystemProgram.programId,
        });
        const keys = [...accounts, ...toAccountMetas(remainingAccounts)];

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data,
        });
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
        );
        /// Encode instruction data
        const rawInputs: InstructionDataInvoke = {
            proof: recentValidityProof,
            inputCompressedAccountsWithMerkleContext:
                packedInputCompressedAccounts,
            outputCompressedAccounts: packedOutputCompressedAccounts,
            relayFee: null,
            newAddressParams: [],
            compressOrDecompressLamports: lamports,
            isCompress: false,
        };
        const data = encodeInstructionDataInvoke(rawInputs);

        const accounts = invokeAccountsLayout({
            ...defaultStaticAccountsStruct(),
            feePayer: payer,
            authority: payer,
            solPoolPda: LightSystemProgram.deriveCompressedSolPda(),
            decompressionRecipient: toAddress,
            systemProgram: SystemProgram.programId,
        });
        const keys = [...accounts, ...toAccountMetas(remainingAccounts)];

        return new TransactionInstruction({
            programId: this.programId,
            keys,
            data,
        });
    }
}
