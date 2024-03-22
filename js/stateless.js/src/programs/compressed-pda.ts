import { Program, AnchorProvider, setProvider, BN } from '@coral-xyz/anchor';
import {
    PublicKey,
    TransactionInstruction,
    Keypair,
    Connection,
} from '@solana/web3.js';
import { IDL, PspCompressedPda } from '../idls/psp_compressed_pda';
import { confirmConfig, defaultStaticAccounts } from '../constants';
import { useWallet } from '../wallet';
import {
    Utxo,
    UtxoWithMerkleContext,
    UtxoWithMerkleProof,
    addMerkleContextToUtxo,
    bn,
    coerceIntoUtxoWithMerkleContext,
    createUtxo,
} from '../state';
import { toArray } from '../utils/conversion';
import { packInstruction } from '../instruction/pack-instruction';
import { pipe } from '../utils/pipe';
import { placeholderValidityProof } from '../instruction/validity-proof';

export type CompressedTransferParams = {
    /** Utxos with lamports to spend as transaction inputs */
    fromBalance: // TODO: selection upfront
    | UtxoWithMerkleContext
        | UtxoWithMerkleProof
        | (UtxoWithMerkleContext | UtxoWithMerkleProof)[];
    /** Solana Account that will receive transferred compressed lamports as utxo  */
    toPubkey: PublicKey;
    /** Amount of compressed lamports to transfer */
    lamports: number | BN;
    // TODO: add
    // /** Optional: if different feepayer than owner of utxos */
    // payer?: PublicKey;
};

/**
 * Create compressed account system transaction params
 */
export type CreateCompressedAccountParams = {
    /*
     * Optional utxos with lamports to spend as transaction inputs.
     * Not required unless 'lamports' are specified, as Light doesn't
     * enforce rent on the protocol level.
     */
    fromBalance: UtxoWithMerkleContext[] | UtxoWithMerkleContext;
    /** Public key of the created account */
    newAccountPubkey: PublicKey;
    /** Amount of lamports to transfer to the created compressed account */
    lamports: number | bigint;
    /** Public key of the program or user to assign as the owner of the created compressed account */
    newAccountOwner: PublicKey;
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
                confirmConfig,
            );
            setProvider(mockProvider);
            this._program = new Program(IDL, this.programId, mockProvider);
        }
    }

    /**
     * Generate a transaction instruction that transfers compressed
     * lamports from one compressed balance to another solana address
     */
    /// TODO: should just define the createoutput utxo selection + packing
    static async transfer(
        params: CompressedTransferParams,
    ): Promise<TransactionInstruction> {
        const recipientUtxo = createUtxo(params.toPubkey, params.lamports);

        // unnecessary if after
        const fromUtxos = pipe(
            toArray<UtxoWithMerkleContext | UtxoWithMerkleProof>,
            coerceIntoUtxoWithMerkleContext,
        )(params.fromBalance);

        const lamports = bn(params.lamports);

        const { selectedAccounts, total } =
            selectMinCompressedAccountsForTransfer(fromUtxos, lamports);

        /// transfer logic
        let changeUtxo: Utxo | undefined = undefined;

        const changeAmount = bn(total).sub(lamports);

        if (bn(changeAmount).gt(bn(0))) {
            changeUtxo = createUtxo(selectedAccounts[0].owner, changeAmount);
        }

        const outputUtxos = changeUtxo
            ? [recipientUtxo, changeUtxo]
            : [recipientUtxo];

        // TODO: move zkp, merkleproof generation, and rootindices outside of transfer
        const recentValidityProof = placeholderValidityProof();
        const recentInputStateRootIndices = selectedAccounts.map(_ => 0);

        const staticAccounts = defaultStaticAccounts();

        const ix = await packInstruction({
            inputState: coerceIntoUtxoWithMerkleContext(selectedAccounts),
            outputState: outputUtxos,
            recentValidityProof,
            recentInputStateRootIndices,
            payer: selectedAccounts[0].owner, // TODO: dynamic payer,
            staticAccounts,
        });
        return ix;
    }
}

//@ts-ignore
if (import.meta.vitest) {
    //@ts-ignore
    const { it, expect, describe } = import.meta.vitest;

    describe('LightSystemProgram.transfer function', () => {
        it('should return a transaction instruction that transfers compressed lamports from one compressed balance to another solana address', async () => {
            const randomPubKeys = [
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(), // 4th
            ];
            const fromBalance = [
                addMerkleContextToUtxo(
                    createUtxo(randomPubKeys[0], bn(1)),
                    bn(0),
                    randomPubKeys[3],
                    0,
                    randomPubKeys[4],
                ),
                addMerkleContextToUtxo(
                    createUtxo(randomPubKeys[0], bn(2)),
                    bn(0),
                    randomPubKeys[3],
                    1,
                    randomPubKeys[4],
                ),
            ];
            const toPubkey = PublicKey.unique();
            const lamports = bn(2);
            const ix = await LightSystemProgram.transfer({
                fromBalance,
                toPubkey,
                lamports,
            });

            console.log('ix', ix.data, ix.data.length);

            expect(ix).toBeDefined();
        });

        it('should throw an error when the input utxos have different owners', async () => {
            const randomPubKeys = [
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
                PublicKey.unique(),
            ];
            const fromBalance = [
                addMerkleContextToUtxo(
                    createUtxo(randomPubKeys[0], bn(1)),
                    bn(0),
                    randomPubKeys[3],
                    0,
                    randomPubKeys[4],
                ),
                addMerkleContextToUtxo(
                    createUtxo(randomPubKeys[1], bn(2)), // diff owner key
                    bn(0),
                    randomPubKeys[3],
                    1,
                    randomPubKeys[4],
                ),
            ];
            const toPubkey = PublicKey.unique();
            const lamports = bn(2);
            await expect(
                LightSystemProgram.transfer({
                    fromBalance,
                    toPubkey,
                    lamports,
                }),
            ).rejects.toThrow('All input utxos must have the same owner');
        });
    });
}

/**
 * @internal
 *
 * Selects the minimal number of compressed accounts for a transfer
 * 1. Sorts the accounts by amount in descending order
 * 2. Accumulates the lamports amount until it is greater than or equal to the transfer
 *    amount
 */
function selectMinCompressedAccountsForTransfer(
    compressedAccounts: (UtxoWithMerkleContext | UtxoWithMerkleProof)[],
    transferAmount: BN,
): {
    selectedAccounts: (UtxoWithMerkleContext | UtxoWithMerkleProof)[];
    total: BN;
} {
    let accumulatedAmount = bn(0);
    const selectedAccounts: (UtxoWithMerkleContext | UtxoWithMerkleProof)[] =
        [];

    compressedAccounts.sort((a, b) =>
        Number(bn(b.lamports).sub(bn(a.lamports))),
    );
    for (const utxo of compressedAccounts) {
        if (accumulatedAmount.gte(bn(transferAmount))) break;
        accumulatedAmount = accumulatedAmount.add(bn(utxo.lamports));
        selectedAccounts.push(utxo);
    }
    if (accumulatedAmount.lt(bn(transferAmount))) {
        throw new Error('Not enough balance for transfer');
    }
    return { selectedAccounts, total: accumulatedAmount };
}
