import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import BN from 'bn.js';
import {
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
    selectStateTreeInfo,
    TreeInfo,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';
import {
    getSplInterfaceInfos,
    selectSplInterfaceInfo,
    SplInterfaceInfo,
} from '../utils/get-token-pool-infos';

/**
 * Mint compressed tokens to a solana address
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Fee payer
 * @param mint                  SPL Mint address
 * @param toPubkey              Address of the account to mint to. Can be an
 *                              array of addresses if the amount is an array of
 *                              amounts.
 * @param authority             Mint authority
 * @param amount                Amount to mint. Pass an array of amounts if the
 *                              toPubkey is an array of addresses.
 * @param outputStateTreeInfo   Optional: State tree account that the compressed
 *                              tokens should be part of. Defaults to the
 *                              default state tree account.
 * @param splInterfaceInfo      Optional: SPL interface information
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function mintTo(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    toPubkey: PublicKey | PublicKey[],
    authority: Signer,
    amount: number | BN | number[] | BN[],
    outputStateTreeInfo?: TreeInfo,
    splInterfaceInfo?: SplInterfaceInfo,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getStateTreeInfos());
    splInterfaceInfo =
        splInterfaceInfo ??
        selectSplInterfaceInfo(await getSplInterfaceInfos(rpc, mint));

    const ix = await CompressedTokenProgram.mintTo({
        feePayer: payer.publicKey,
        mint,
        authority: authority.publicKey,
        amount,
        toPubkey,
        outputStateTreeInfo,
        tokenPoolInfo: splInterfaceInfo,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [authority]);

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return sendAndConfirmTx(rpc, tx, confirmOptions);
}
