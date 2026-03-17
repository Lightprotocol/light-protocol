import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import {
    PublicKey,
    Signer,
    TransactionSignature,
    ConfirmOptions,
} from '@solana/web3.js';
import { createLoadAtaInstructions } from '../instructions/load-ata';
import { InterfaceOptions } from './transfer-interface';
import { getMintInterface } from '../get-mint-interface';

export {
    createLoadAtaInstructions,
    selectInputsForAmount,
    getCompressedTokenAccountsFromAtaSources,
    MAX_INPUT_ACCOUNTS,
    InternalLoadBatch,
    rawLoadBatchComputeUnits,
    calculateLoadBatchComputeUnits,
    _buildLoadBatches,
    AtaType,
} from '../instructions/load-ata';

export async function loadAta(
    rpc: Rpc,
    ata: PublicKey,
    owner: Signer,
    mint: PublicKey,
    payer?: Signer,
    confirmOptions?: ConfirmOptions,
    interfaceOptions?: InterfaceOptions,
    wrap = false,
    decimals?: number,
): Promise<TransactionSignature | null> {
    assertBetaEnabled();

    payer ??= owner;

    const resolvedDecimals =
        decimals ?? (await getMintInterface(rpc, mint)).mint.decimals;
    const batches = await createLoadAtaInstructions(
        rpc,
        ata,
        owner.publicKey,
        mint,
        resolvedDecimals,
        payer.publicKey,
        interfaceOptions,
        wrap,
    );

    if (batches.length === 0) {
        return null;
    }

    const additionalSigners = dedupeSigner(payer, [owner]);

    const txPromises = batches.map(async ixs => {
        const { blockhash } = await rpc.getLatestBlockhash();
        const tx = buildAndSignTx(ixs, payer!, blockhash, additionalSigners);
        return sendAndConfirmTx(rpc, tx, confirmOptions);
    });

    const results = await Promise.all(txPromises);
    return results[results.length - 1];
}
