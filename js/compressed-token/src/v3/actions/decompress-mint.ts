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
    DerivationMode,
    bn,
    LIGHT_TOKEN_PROGRAM_ID,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import { createDecompressMintInstruction } from '../instructions/decompress-mint';
import { getMintInterface } from '../get-mint-interface';

export interface DecompressMintParams {
    /** Number of epochs to prepay rent (minimum 2, default: 16 for ~24 hours) */
    rentPayment?: number;
    /** Per-write top-up in lamports (default: 766 for ~2 epochs) */
    writeTopUp?: number;
    /** Compressible config account (default: LIGHT_TOKEN_CONFIG) */
    configAccount?: PublicKey;
    /** Rent sponsor PDA (default: LIGHT_TOKEN_RENT_SPONSOR) */
    rentSponsor?: PublicKey;
    /** Cap on rent top-up for this instruction (units of 1k lamports; default no cap) */
    maxTopUp?: number;
}

/**
 * Decompress a compressed light mint to create the light mint account.
 *
 * This creates the light mint account, which is required before creating
 * light-token associated token accounts. DecompressMint is **permissionless** -
 * any account can call it.
 *
 * @param rpc - RPC connection
 * @param payer - Fee payer (signer)
 * @param mint - Mint address
 * @param authority - Authority signer (can be any account, required for MintAction)
 * @param params - Optional decompression parameters
 * @param confirmOptions - Optional confirm options
 * @returns Transaction signature
 */
export async function decompressMint(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    authority?: Signer,
    params?: DecompressMintParams,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    // Use payer as authority if not provided (decompressMint is permissionless)
    const effectiveAuthority = authority ?? payer;

    const mintInterface = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        LIGHT_TOKEN_PROGRAM_ID,
    );

    if (!mintInterface.merkleContext) {
        throw new Error('Mint does not have MerkleContext');
    }

    // Already decompressed (e.g. createMintInterface now does it atomically).
    // Return early instead of throwing so callers are idempotent.
    if (mintInterface.mintContext?.cmintDecompressed) {
        return '' as TransactionSignature;
    }

    const validityProof = await rpc.getValidityProofV2(
        [
            {
                hash: bn(mintInterface.merkleContext.hash),
                leafIndex: mintInterface.merkleContext.leafIndex,
                treeInfo: mintInterface.merkleContext.treeInfo,
                proveByIndex: mintInterface.merkleContext.proveByIndex,
            },
        ],
        [],
        DerivationMode.compressible,
    );

    const ix = createDecompressMintInstruction({
        mintInterface,
        authority: effectiveAuthority.publicKey,
        payer: payer.publicKey,
        validityProof,
        rentPayment: params?.rentPayment,
        writeTopUp: params?.writeTopUp,
        configAccount: params?.configAccount,
        rentSponsor: params?.rentSponsor,
        maxTopUp: params?.maxTopUp,
    });

    const additionalSigners: Signer[] = [];
    if (authority && !effectiveAuthority.publicKey.equals(payer.publicKey)) {
        additionalSigners.push(effectiveAuthority);
    }

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}
