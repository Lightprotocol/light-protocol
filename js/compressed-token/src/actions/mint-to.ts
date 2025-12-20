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

function isBatchNotReadyError(error: unknown): boolean {
    const message = error instanceof Error ? error.message : String(error);
    // BatchedMerkleTreeError::BatchNotReady (14301) => custom program error 0x37dd
    return (
        message.includes('0x37dd') ||
        message.includes('14301') ||
        message.includes('BatchNotReady')
    );
}

async function selectAlternativeStateTreeInfo(
    rpc: Rpc,
    current: TreeInfo,
): Promise<TreeInfo> {
    const infos = await rpc.getStateTreeInfos();

    // Prefer a different active tree of the same type.
    const candidates = infos.filter(
        t =>
            t.treeType === current.treeType &&
            !t.nextTreeInfo &&
            !t.queue.equals(current.queue),
    );

    if (candidates.length > 0) {
        const length = Math.min(5, candidates.length);
        return candidates[Math.floor(Math.random() * length)];
    }

    // Fall back to normal selection (may return the same tree).
    return selectStateTreeInfo(infos, current.treeType, true);
}

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

    // Retry on BatchNotReady (full output queue batch) by selecting a different
    // active state tree. This can happen under heavy test load when one of the
    // V2 output queues becomes blocked.
    let selectedTree = outputStateTreeInfo;
    let lastError: unknown;
    for (let attempt = 0; attempt < 3; attempt++) {
        try {
            const ix = await CompressedTokenProgram.mintTo({
                feePayer: payer.publicKey,
                mint,
                authority: authority.publicKey,
                amount,
                toPubkey,
                outputStateTreeInfo: selectedTree,
                tokenPoolInfo: splInterfaceInfo,
            });

            const { blockhash } = await rpc.getLatestBlockhash();
            const additionalSigners = dedupeSigner(payer, [authority]);

            const tx = buildAndSignTx(
                [
                    ComputeBudgetProgram.setComputeUnitLimit({
                        units: 1_000_000,
                    }),
                    ix,
                ],
                payer,
                blockhash,
                additionalSigners,
            );

            return sendAndConfirmTx(rpc, tx, confirmOptions);
        } catch (error) {
            lastError = error;
            if (!isBatchNotReadyError(error) || attempt === 2) {
                throw error;
            }
            selectedTree = await selectAlternativeStateTreeInfo(
                rpc,
                selectedTree,
            );
        }
    }

    // Unreachable, but keeps TS happy.
    throw lastError;
}
