import { Program, AnchorProvider, setProvider, BN } from '@coral-xyz/anchor';
import { PublicKey, Keypair, Connection } from '@solana/web3.js';
import { IDL, PspCompressedPda } from '../idls/psp_compressed_pda';
import { confirmConfig } from '../constants';
import { useWallet } from '../wallet';
import { UtxoWithMerkleContext, UtxoWithMerkleProof, bn } from '../state';

/// TODO: add transfer
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
}

/**
 * @internal
 *
 * Selects the minimal number of compressed accounts for a transfer
 * 1. Sorts the accounts by amount in descending order
 * 2. Accumulates the lamports amount until it is greater than or equal to the transfer
 *    amount
 */
function _selectMinCompressedAccountsForTransfer(
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
