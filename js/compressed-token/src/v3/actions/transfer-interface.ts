import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
    assertBetaEnabled,
    LIGHT_TOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import { createTransferInterfaceInstructions } from '../instructions/transfer-interface';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { type SplInterfaceInfo } from '../../utils/get-token-pool-infos';

export interface InterfaceOptions {
    splInterfaceInfos?: SplInterfaceInfo[];
    /**
     * ATA owner (authority owner) used to derive the ATA when the signer is a
     * delegate. For owner-signed flows, omit this field.
     */
    owner?: PublicKey;
}

export async function transferInterface(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    owner: Signer,
    amount: number | bigint | BN,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
    confirmOptions?: ConfirmOptions,
    options?: InterfaceOptions,
    wrap = false,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const effectiveOwner = options?.owner ?? owner.publicKey;
    const expectedSource = getAssociatedTokenAddressInterface(
        mint,
        effectiveOwner,
        false,
        programId,
    );
    if (!source.equals(expectedSource)) {
        throw new Error(
            `Source mismatch. Expected ${expectedSource.toBase58()}, got ${source.toBase58()}`,
        );
    }

    const amountBigInt = BigInt(amount.toString());

    const batches = await createTransferInterfaceInstructions(
        rpc,
        payer.publicKey,
        mint,
        amountBigInt,
        owner.publicKey,
        destination,
        {
            ...options,
            wrap,
            programId,
            ensureRecipientAta: true,
            owner: options?.owner,
        },
    );

    const additionalSigners = dedupeSigner(payer, [owner]);
    const { rest: loads, last: transferIxs } = sliceLast(batches);

    await Promise.all(
        loads.map(async ixs => {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(ixs, payer, blockhash, additionalSigners);
            return sendAndConfirmTx(rpc, tx, confirmOptions);
        }),
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(transferIxs, payer, blockhash, additionalSigners);
    return sendAndConfirmTx(rpc, tx, confirmOptions);
}

export interface TransferOptions extends InterfaceOptions {
    wrap?: boolean;
    programId?: PublicKey;
    ensureRecipientAta?: boolean;
}

export function sliceLast<T>(items: T[]): { rest: T[]; last: T } {
    if (items.length === 0) {
        throw new Error('sliceLast: array must not be empty');
    }
    return { rest: items.slice(0, -1), last: items.at(-1)! };
}

export {
    createTransferInterfaceInstructions,
    calculateTransferCU,
} from '../instructions/transfer-interface';
