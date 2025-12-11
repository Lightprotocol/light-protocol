import {
    ComputeBudgetProgram,
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
} from '@lightprotocol/stateless.js';
import { createWrapInstruction } from '../instructions/wrap';
import {
    getSplInterfaceInfos,
    SplInterfaceInfo,
} from '../../utils/get-token-pool-infos';

/**
 * Wrap tokens from an SPL/T22 account to a c-token account.
 *
 * This is an agnostic action that takes explicit account addresses (spl-token style).
 * Use getAssociatedTokenAddressSync() to derive ATA addresses if needed.
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer
 * @param source          Source SPL/T22 token account (any token account, not just ATA)
 * @param destination     Destination c-token account
 * @param owner           Owner/authority of the source account (must sign)
 * @param mint            Mint address
 * @param amount          Amount to wrap
 * @param splInterfaceInfo Optional: SPL interface info (will be fetched if not provided)
 * @param confirmOptions  Optional: Confirm options
 *
 * @example
 * const splAta = getAssociatedTokenAddressSync(mint, owner.publicKey, false, TOKEN_PROGRAM_ID);
 * const ctokenAta = getAssociatedTokenAddressInterface(mint, owner.publicKey); // defaults to c-token
 *
 * await wrap(
 *   rpc,
 *   payer,
 *   splAta,
 *   ctokenAta,
 *   owner,
 *   mint,
 *   1000n,
 * );
 *
 * @returns Transaction signature
 */
export async function wrap(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    destination: PublicKey,
    owner: Signer,
    mint: PublicKey,
    amount: bigint,
    splInterfaceInfo?: SplInterfaceInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    // Get SPL interface info if not provided
    let resolvedSplInterfaceInfo = splInterfaceInfo;
    if (!resolvedSplInterfaceInfo) {
        const splInterfaceInfos = await getSplInterfaceInfos(rpc, mint);
        resolvedSplInterfaceInfo = splInterfaceInfos.find(
            info => info.isInitialized,
        );

        if (!resolvedSplInterfaceInfo) {
            throw new Error(
                `No initialized SPL interface found for mint: ${mint.toBase58()}. ` +
                    `Please create an SPL interface via createSplInterface().`,
            );
        }
    }

    // Build wrap instruction
    const ix = createWrapInstruction(
        source,
        destination,
        owner.publicKey,
        mint,
        amount,
        resolvedSplInterfaceInfo,
        payer.publicKey,
    );

    // Build and send transaction
    const { blockhash } = await rpc.getLatestBlockhash();

    const additionalSigners = dedupeSigner(payer, [owner]);

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}
