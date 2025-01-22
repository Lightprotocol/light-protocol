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
    pickRandomTreeAndQueue,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';

/**
 * Mint compressed tokens to a solana address
 *
 * @param rpc            Rpc to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint for the account
 * @param destination    Address of the account to mint to. Can be an array of
 *                       addresses if the amount is an array of amounts.
 * @param authority      Minting authority
 * @param amount         Amount to mint. Can be an array of amounts if the
 *                       destination is an array of addresses.
 * @param merkleTree     State tree account that the compressed tokens should be
 *                       part of. Defaults to the default state tree account.
 * @param confirmOptions Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function mintTo(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    destination: PublicKey | PublicKey[],
    authority: Signer,
    amount: number | BN | number[] | BN[],
    merkleTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
    tokenProgramId?: PublicKey,
): Promise<TransactionSignature> {
    tokenProgramId = tokenProgramId
        ? tokenProgramId
        : await CompressedTokenProgram.get_mint_program_id(mint, rpc);

    const additionalSigners = dedupeSigner(payer, [authority]);

    if (!merkleTree) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfo();
        const { tree } = pickRandomTreeAndQueue(stateTreeInfo);
        merkleTree = tree;
    }

    const ix = await CompressedTokenProgram.mintTo({
        feePayer: payer.publicKey,
        mint,
        authority: authority.publicKey,
        amount: amount,
        toPubkey: destination,
        merkleTree,
        tokenProgramId,
    });

    const { blockhash } = await rpc.getLatestBlockhash();

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}
