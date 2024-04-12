import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import {
    defaultTestStateTreeAccounts,
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';
import { dedupeSigner, getSigners } from './common';

/**
 * Mint compressed tokens to a solana address
 *
 * @param rpc            Rpc to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint for the account
 * @param destination    Address of the account to mint to
 * @param authority      Minting authority
 * @param amount         Amount to mint
 * @param multiSigners   Signing accounts if `authority` is a multisig
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
    destination: PublicKey,
    authority: Signer | PublicKey,
    amount: number | BN,
    multiSigners: Signer[] = [],
    merkleTree: PublicKey = defaultTestStateTreeAccounts().merkleTree, // DEFAULT IF NOT PROVIDED
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const [authorityPubkey, authoritySigners] = getSigners(
        authority,
        multiSigners,
    );
    const additionalSigners = dedupeSigner(payer, authoritySigners);

    const ix = await CompressedTokenProgram.mintTo({
        feePayer: payer.publicKey,
        mint,
        authority: authorityPubkey,
        amount: amount,
        toPubkey: destination,
        merkleTree,
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
