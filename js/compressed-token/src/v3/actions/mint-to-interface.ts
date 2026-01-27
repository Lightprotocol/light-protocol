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
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import { createMintToInterfaceInstruction } from '../instructions/mint-to-interface';
import { getMintInterface } from '../get-mint-interface';

/**
 * Mint tokens to a decompressed/onchain token account.
 * Works with SPL, Token-2022, and compressed token (c-token) mints.
 *
 * This function ONLY mints to decompressed onchain token accounts, never to compressed accounts.
 * The signature matches the standard SPL mintTo for simplicity and consistency.
 *
 * @param rpc - RPC connection to use
 * @param payer - Transaction fee payer
 * @param mint - Mint address (SPL, Token-2022, or compressed mint)
 * @param destination - Destination token account address (must be an existing onchain token account)
 * @param authority - Mint authority (can be Signer or PublicKey if multiSigners provided)
 * @param amount - Amount to mint
 * @param multiSigners - Optional: Multi-signature signers (default: [])
 * @param confirmOptions - Optional: Transaction confirmation options
 * @param programId - Optional: Token program ID (TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID, or CTOKEN_PROGRAM_ID). If undefined, auto-detects.
 *
 * @returns Transaction signature
 */
export async function mintToInterface(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    destination: PublicKey,
    authority: Signer | PublicKey,
    amount: number | bigint,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId?: PublicKey,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    // Fetch mint interface (auto-detects program type if not provided)
    const mintInterface = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        programId,
    );

    // Fetch validity proof if this is a compressed mint (has merkleContext)
    let validityProof;
    if (mintInterface.merkleContext) {
        validityProof = await rpc.getValidityProofV2(
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
    }

    // Create instruction
    const authorityPubkey =
        authority instanceof PublicKey ? authority : authority.publicKey;
    const multiSignerPubkeys = multiSigners.map(s => s.publicKey);

    const ix = createMintToInterfaceInstruction(
        mintInterface,
        destination,
        authorityPubkey,
        payer.publicKey,
        amount,
        validityProof,
        multiSignerPubkeys,
    );

    // Build signers list
    const signers: Signer[] = [];
    if (authority instanceof PublicKey) {
        // Authority is a pubkey, so multiSigners must be provided
        signers.push(...multiSigners);
    } else {
        // Authority is a signer
        if (!authority.publicKey.equals(payer.publicKey)) {
            signers.push(authority);
        }
        signers.push(...multiSigners);
    }

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        signers,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}
