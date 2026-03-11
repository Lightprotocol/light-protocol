import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionInstruction,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import { sliceLast } from './transfer-interface';
import BN from 'bn.js';
import { createUnwrapInstructions } from '../instructions/unwrap';
import { getMintInterface } from '../get-mint-interface';
import { type SplInterfaceInfo } from '../../utils/get-token-pool-infos';

export { createUnwrapInstructions } from '../instructions/unwrap';

export async function unwrap(
    rpc: Rpc,
    payer: Signer,
    destination: PublicKey,
    owner: Signer,
    mint: PublicKey,
    amount?: number | bigint | BN,
    splInterfaceInfo?: SplInterfaceInfo,
    maxTopUp?: number,
    confirmOptions?: ConfirmOptions,
    decimals?: number,
    wrap = false,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const resolvedDecimals =
        decimals ?? (await getMintInterface(rpc, mint)).mint.decimals;
    const batches = await createUnwrapInstructions(
        rpc,
        destination,
        owner.publicKey,
        mint,
        resolvedDecimals,
        amount,
        payer.publicKey,
        splInterfaceInfo,
        maxTopUp,
        undefined,
        wrap,
    );

    const additionalSigners = dedupeSigner(payer, [owner]);
    const { rest: loads, last: unwrapIxs } = sliceLast(batches);

    await Promise.all(
        loads.map(async (ixs: TransactionInstruction[]) => {
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(ixs, payer, blockhash, additionalSigners);
            return sendAndConfirmTx(rpc, tx, confirmOptions);
        }),
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(unwrapIxs, payer, blockhash, additionalSigners);
    return sendAndConfirmTx(rpc, tx, confirmOptions);
}
