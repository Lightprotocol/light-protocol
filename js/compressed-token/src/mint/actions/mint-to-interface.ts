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
    CTOKEN_PROGRAM_ID,
    selectStateTreeInfo,
    TreeInfo,
} from '@lightprotocol/stateless.js';
import { TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';
import { createMintToInterfaceInstruction } from '../instructions/mint-to-interface';
import { getMintInterface } from '../helpers';
import { mintSplTo } from '../../actions/mint-spl-to';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../utils/get-token-pool-infos';

/**
 * Mint tokens to accounts - works with both SPL and compressed mints
 *
 * For SPL mints: Mints to compressed token accounts (requires state tree and token pool)
 * For compressed mints: Mints to either compressed accounts or onchain ctoken accounts
 *
 * @param rpc - RPC connection to use
 * @param payer - Fee payer
 * @param mint - Mint address (SPL or compressed)
 * @param recipient - Recipient address(es). For SPL: owner pubkey(s). For compressed: account address or owner pubkey(s)
 * @param authority - Mint authority
 * @param amount - Amount(s) to mint
 * @param outputStateTreeInfo - Optional: State tree info (required for SPL mints)
 * @param tokenPoolInfo - Optional: Token pool info (required for SPL mints)
 * @param outputQueue - Optional: Output queue (for compressed mints)
 * @param tokensOutQueue - Optional: Tokens output queue (for compressed mints)
 * @param tokenAccountVersion - Optional: Token account version (for compressed mints, default: 3)
 * @param confirmOptions - Options for confirming the transaction
 *
 * @returns Transaction signature
 */
export async function mintToInterface(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    recipient: PublicKey | PublicKey[],
    authority: Signer,
    amount: number | bigint | Array<number | bigint>,
    outputStateTreeInfo?: TreeInfo,
    tokenPoolInfo?: TokenPoolInfo,
    outputQueue?: PublicKey,
    tokensOutQueue?: PublicKey,
    tokenAccountVersion: number = 3,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    // Auto-detect mint type by trying to fetch it
    const mintInfo = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
    );

    // SPL Token mints (no merkleContext means SPL)
    if (!mintInfo.merkleContext) {
        // Resolve state tree and token pool if not provided
        const resolvedOutputStateTreeInfo =
            outputStateTreeInfo ??
            selectStateTreeInfo(await rpc.getStateTreeInfos());
        const resolvedTokenPoolInfo =
            tokenPoolInfo ??
            selectTokenPoolInfo(await getTokenPoolInfos(rpc, mint));

        // Convert to BN/number types expected by mintSplTo
        return await mintSplTo(
            rpc,
            payer,
            mint,
            recipient,
            authority,
            amount as any,
            resolvedOutputStateTreeInfo,
            resolvedTokenPoolInfo,
            confirmOptions,
        );
    }

    // Compressed mint (has merkleContext)
    // Resolve queues
    const resolvedOutputQueue =
        outputQueue ||
        (outputStateTreeInfo?.queue ??
            (await rpc.getStateTreeInfos())[0].queue);
    const resolvedTokensOutQueue = tokensOutQueue || resolvedOutputQueue;

    const validityProof = await rpc.getValidityProofV2(
        [
            {
                hash: bn(mintInfo.merkleContext.hash),
                leafIndex: mintInfo.merkleContext.leafIndex,
                treeInfo: mintInfo.merkleContext.treeInfo,
                proveByIndex: mintInfo.merkleContext.proveByIndex,
            },
        ],
        [],
        DerivationMode.compressible,
    );

    const mintDataForInstruction = {
        supply: mintInfo.mint.supply,
        decimals: mintInfo.mint.decimals,
        mintAuthority: mintInfo.mint.mintAuthority,
        freezeAuthority: mintInfo.mint.freezeAuthority,
        splMint: mintInfo.mintContext!.splMint,
        splMintInitialized: mintInfo.mintContext!.splMintInitialized,
        version: mintInfo.mintContext!.version,
        metadata: mintInfo.tokenMetadata
            ? {
                  updateAuthority:
                      mintInfo.tokenMetadata.updateAuthority || null,
                  name: mintInfo.tokenMetadata.name,
                  symbol: mintInfo.tokenMetadata.symbol,
                  uri: mintInfo.tokenMetadata.uri,
              }
            : undefined,
    };

    const ix = createMintToInterfaceInstruction(
        mint,
        authority.publicKey,
        payer.publicKey,
        recipient,
        amount,
        validityProof,
        mintInfo.merkleContext,
        mintDataForInstruction,
        resolvedOutputQueue,
        resolvedTokensOutQueue,
        tokenAccountVersion,
    );

    const additionalSigners = authority.publicKey.equals(payer.publicKey)
        ? []
        : [authority];

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}
