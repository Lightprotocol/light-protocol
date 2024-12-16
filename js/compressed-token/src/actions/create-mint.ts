import {
    ConfirmOptions,
    Keypair,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import { CompressedTokenProgram } from '../program';
import {
    MINT_SIZE,
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
} from '@lightprotocol/stateless.js';

/**
 * Create and initialize a new compressed token mint
 *
 * @param rpc             RPC to use
 * @param payer           Payer of the transaction and initialization fees
 * @param mintAuthority   Account or multisig that will control minting
 * @param decimals        Location of the decimal place
 * @param keypair         Optional keypair, defaulting to a new random one
 * @param confirmOptions  Options for confirming the transaction
 * @param isToken22       Whether to create a Token 2022 mint. Defaults to false.
 *
 * @return Address of the new mint and the transaction signature
 */
export async function createMint(
    rpc: Rpc,
    payer: Signer,
    mintAuthority: PublicKey,
    decimals: number,
    keypair = Keypair.generate(),
    confirmOptions?: ConfirmOptions,
    isToken22 = false,
): Promise<{ mint: PublicKey; transactionSignature: TransactionSignature }> {
    const rentExemptBalance =
        await rpc.getMinimumBalanceForRentExemption(MINT_SIZE);

    const tokenProgramId = isToken22 ? TOKEN_2022_PROGRAM_ID : TOKEN_PROGRAM_ID;

    const ixs = await CompressedTokenProgram.createMint({
        feePayer: payer.publicKey,
        mint: keypair.publicKey,
        decimals,
        authority: mintAuthority,
        freezeAuthority: null, // TODO: add feature
        rentExemptBalance,
        tokenProgramId,
    });

    const { blockhash } = await rpc.getLatestBlockhash();

    const additionalSigners = dedupeSigner(payer, [keypair]);

    const tx = buildAndSignTx(ixs, payer, blockhash, additionalSigners);

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return { mint: keypair.publicKey, transactionSignature: txId };
}
