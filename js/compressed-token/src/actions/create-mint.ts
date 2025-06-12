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
} from '@lightprotocol/stateless.js';

/**
 * Create and initialize a new compressed token mint
 *
 * @param rpc               RPC connection to use
 * @param payer             Fee payer
 * @param mintAuthority     Account that will control minting
 * @param decimals          Location of the decimal place
 * @param keypair           Optional: Mint keypair. Defaults to a random
 *                          keypair.
 * @param confirmOptions    Options for confirming the transaction
 * @param tokenProgramId    Optional: Program ID for the token. Defaults to
 *                          TOKEN_PROGRAM_ID.
 * @param freezeAuthority   Optional: Account that will control freeze and thaw.
 *                          Defaults to none.
 *
 * @return Object with mint address and transaction signature
 */
export async function createMint(
    rpc: Rpc,
    payer: Signer,
    mintAuthority: PublicKey | Signer,
    decimals: number,
    keypair = Keypair.generate(),
    confirmOptions?: ConfirmOptions,
    tokenProgramId?: PublicKey | boolean,
    freezeAuthority?: PublicKey | Signer,
): Promise<{ mint: PublicKey; transactionSignature: TransactionSignature }> {
    const rentExemptBalance =
        await rpc.getMinimumBalanceForRentExemption(MINT_SIZE);

    // If true, uses TOKEN_2022_PROGRAM_ID.
    // If false or undefined, defaults to TOKEN_PROGRAM_ID.
    // Otherwise, uses the provided tokenProgramId.
    const resolvedTokenProgramId =
        tokenProgramId === true
            ? TOKEN_2022_PROGRAM_ID
            : tokenProgramId || TOKEN_PROGRAM_ID;

    const ixs = await CompressedTokenProgram.createMint({
        feePayer: payer.publicKey,
        mint: keypair.publicKey,
        decimals,
        authority: getPublicKey(mintAuthority)!,
        freezeAuthority: getPublicKey(freezeAuthority),
        rentExemptBalance,
        tokenProgramId: resolvedTokenProgramId,
    });

    const { blockhash } = await rpc.getLatestBlockhash();

    // Get required additional signers that are Keypairs, not the payer.
    const additionalSigners = [mintAuthority, freezeAuthority]
        .filter(
            (signer): signer is Signer =>
                signer instanceof Keypair &&
                !signer.publicKey.equals(payer.publicKey),
        )
        .filter(
            (signer, index, array) =>
                array.findIndex(s => s.publicKey.equals(signer.publicKey)) ===
                index,
        );

    const tx = buildAndSignTx(ixs, payer, blockhash, [
        ...additionalSigners,
        keypair,
    ]);
    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return { mint: keypair.publicKey, transactionSignature: txId };
}

const getPublicKey = (
    signer: PublicKey | Signer | undefined,
): PublicKey | null =>
    signer instanceof PublicKey ? signer : signer?.publicKey || null;
