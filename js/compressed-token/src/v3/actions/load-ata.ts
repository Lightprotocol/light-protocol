import {
    Rpc,
    CTOKEN_PROGRAM_ID,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
    ParsedTokenAccount,
    bn,
} from '@lightprotocol/stateless.js';
import { assertV2Only } from '../assert-v2-only';
import {
    PublicKey,
    TransactionInstruction,
    Signer,
    TransactionSignature,
    ConfirmOptions,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
    createAssociatedTokenAccountIdempotentInstruction,
    getMint,
    TokenAccountNotFoundError,
} from '@solana/spl-token';
import {
    AccountInterface,
    getAtaInterface as _getAtaInterface,
    TokenAccountSource,
    TokenAccountSourceType,
} from '../get-account-interface';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../instructions/create-ata-interface';
import { createWrapInstruction } from '../instructions/wrap';
import { createDecompressInterfaceInstruction } from '../instructions/create-decompress-interface-instruction';
import {
    getSplInterfaceInfos,
    SplInterfaceInfo,
} from '../../utils/get-token-pool-infos';
import { getAtaProgramId, checkAtaAddress, AtaType } from '../ata-utils';
import { InterfaceOptions } from './transfer-interface';

function getCompressedTokenAccountsFromAtaSources(
    sources: TokenAccountSource[],
): ParsedTokenAccount[] {
    const coldTypes = new Set<TokenAccountSource['type']>([
        TokenAccountSourceType.CTokenCold,
        TokenAccountSourceType.SplCold,
        TokenAccountSourceType.Token2022Cold,
    ]);

    return sources
        .filter(source => source.loadContext !== undefined)
        .filter(source => coldTypes.has(source.type))
        .map(source => {
            const fullData = source.accountInfo.data;
            const discriminatorBytes = fullData.subarray(
                0,
                Math.min(8, fullData.length),
            );
            const accountDataBytes =
                fullData.length > 8 ? fullData.subarray(8) : Buffer.alloc(0);

            const compressedAccount = {
                treeInfo: source.loadContext!.treeInfo,
                hash: source.loadContext!.hash,
                leafIndex: source.loadContext!.leafIndex,
                proveByIndex: source.loadContext!.proveByIndex,
                owner: source.accountInfo.owner,
                lamports: bn(source.accountInfo.lamports),
                address: null,
                data:
                    fullData.length === 0
                        ? null
                        : {
                              discriminator: Array.from(discriminatorBytes),
                              data: Buffer.from(accountDataBytes),
                              dataHash: new Array(32).fill(0),
                          },
                readOnly: false,
            };

            const state = !source.parsed.isInitialized
                ? 0
                : source.parsed.isFrozen
                  ? 2
                  : 1;

            return {
                compressedAccount: compressedAccount as any,
                parsed: {
                    mint: source.parsed.mint,
                    owner: source.parsed.owner,
                    amount: bn(source.parsed.amount.toString()),
                    delegate: source.parsed.delegate,
                    state,
                    tlv:
                        source.parsed.tlvData.length > 0
                            ? source.parsed.tlvData
                            : null,
                },
            } satisfies ParsedTokenAccount;
        });
}

// Re-export types moved to instructions
export {
    ParsedAccountInfoInterface,
    CompressibleAccountInput,
    PackedCompressedAccount,
    CompressibleLoadParams,
    LoadResult,
    createLoadAccountsParams,
    calculateCompressibleLoadComputeUnits,
} from '../instructions/create-load-accounts-params';

/**
 * Create instructions to load token balances into an ATA.
 *
 * Behavior depends on `wrap` parameter:
 * - wrap=false (standard): Decompress compressed tokens to the target ATA.
 *   ATA can be SPL (via pool), T22 (via pool), or c-token (direct).
 * - wrap=true (unified): Wrap SPL/T22 + decompress all to c-token ATA.
 *   ATA must be a c-token ATA.
 *
 * @param rpc     RPC connection
 * @param ata     Associated token address (SPL, T22, or c-token)
 * @param owner   Owner public key
 * @param mint    Mint public key
 * @param payer   Fee payer (defaults to owner)
 * @param options Optional load options
 * @param wrap    Unified mode: wrap SPL/T22 to c-token (default: false)
 * @returns       Array of instructions (empty if nothing to load)
 */
export async function createLoadAtaInstructions(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    payer?: PublicKey,
    options?: InterfaceOptions,
    wrap = false,
): Promise<TransactionInstruction[]> {
    payer ??= owner;

    // Validation happens inside getAtaInterface via checkAtaAddress helper:
    // - Always validates ata matches mint+owner derivation
    // - For wrap=true, additionally requires c-token ATA
    try {
        const ataInterface = await _getAtaInterface(
            rpc,
            ata,
            owner,
            mint,
            undefined,
            undefined,
            wrap,
        );
        return createLoadAtaInstructionsFromInterface(
            rpc,
            payer,
            ataInterface,
            options,
            wrap,
            ata,
        );
    } catch (error) {
        // If account doesn't exist, there's nothing to load
        if (error instanceof TokenAccountNotFoundError) {
            return [];
        }
        throw error;
    }
}

// Re-export AtaType for backwards compatibility
export { AtaType } from '../ata-utils';

/**
 * Create instructions to load an ATA from its AccountInterface.
 *
 * Behavior depends on `wrap` parameter:
 * - wrap=false (standard): Decompress compressed tokens to the target ATA type
 *   (SPL ATA via pool, T22 ATA via pool, or c-token ATA direct)
 * - wrap=true (unified): Wrap SPL/T22 + decompress all to c-token ATA
 *
 * @param rpc         RPC connection
 * @param payer       Fee payer
 * @param ata         AccountInterface from getAtaInterface (must have _isAta, _owner, _mint)
 * @param options     Optional load options
 * @param wrap        Unified mode: wrap SPL/T22 to c-token (default: false)
 * @param targetAta   Target ATA address (used for type detection in standard mode)
 * @returns           Array of instructions (empty if nothing to load)
 */
export async function createLoadAtaInstructionsFromInterface(
    rpc: Rpc,
    payer: PublicKey,
    ata: AccountInterface,
    options?: InterfaceOptions,
    wrap = false,
    targetAta?: PublicKey,
): Promise<TransactionInstruction[]> {
    if (!ata._isAta || !ata._owner || !ata._mint) {
        throw new Error(
            'AccountInterface must be from getAtaInterface (requires _isAta, _owner, _mint)',
        );
    }

    const instructions: TransactionInstruction[] = [];
    const owner = ata._owner;
    const mint = ata._mint;
    const sources = ata._sources ?? [];

    // v3 interface only supports V2 trees - check cold sources early
    const compressedAccountsToCheck =
        getCompressedTokenAccountsFromAtaSources(sources);
    if (compressedAccountsToCheck.length > 0) {
        assertV2Only(compressedAccountsToCheck);
    }

    // Derive addresses
    const ctokenAtaAddress = getAssociatedTokenAddressInterface(mint, owner);
    const splAta = getAssociatedTokenAddressSync(
        mint,
        owner,
        false,
        TOKEN_PROGRAM_ID,
        getAtaProgramId(TOKEN_PROGRAM_ID),
    );
    const t22Ata = getAssociatedTokenAddressSync(
        mint,
        owner,
        false,
        TOKEN_2022_PROGRAM_ID,
        getAtaProgramId(TOKEN_2022_PROGRAM_ID),
    );

    // Validate and detect target ATA type
    // If called via createLoadAtaInstructions, validation already happened in getAtaInterface.
    // If called directly, this validates the targetAta is correct.
    let ataType: AtaType = 'ctoken';
    if (targetAta) {
        const validation = checkAtaAddress(targetAta, mint, owner);
        ataType = validation.type;

        // For wrap=true, must be c-token ATA
        if (wrap && ataType !== 'ctoken') {
            throw new Error(
                `For wrap=true, targetAta must be c-token ATA. Got ${ataType} ATA.`,
            );
        }
    }

    // Check sources for balances
    const splSource = sources.find(s => s.type === 'spl');
    const t22Source = sources.find(s => s.type === 'token2022');
    const ctokenHotSource = sources.find(s => s.type === 'ctoken-hot');
    const ctokenColdSource = sources.find(s => s.type === 'ctoken-cold');

    const splBalance = splSource?.amount ?? BigInt(0);
    const t22Balance = t22Source?.amount ?? BigInt(0);
    const coldBalance = ctokenColdSource?.amount ?? BigInt(0);

    // Nothing to load
    if (
        splBalance === BigInt(0) &&
        t22Balance === BigInt(0) &&
        coldBalance === BigInt(0)
    ) {
        return [];
    }

    // Get SPL interface info (needed for wrapping or SPL/T22 decompression)
    let splInterfaceInfo: SplInterfaceInfo | undefined;
    const needsSplInfo =
        wrap ||
        ataType === 'spl' ||
        ataType === 'token2022' ||
        splBalance > BigInt(0) ||
        t22Balance > BigInt(0);

    let decimals = 0;
    if (needsSplInfo) {
        try {
            const splInterfaceInfos =
                options?.splInterfaceInfos ??
                (await getSplInterfaceInfos(rpc, mint));
            splInterfaceInfo = splInterfaceInfos.find(
                (info: SplInterfaceInfo) => info.isInitialized,
            );
            if (splInterfaceInfo) {
                const mintInfo = await getMint(
                    rpc,
                    mint,
                    undefined,
                    splInterfaceInfo.tokenProgram,
                );
                decimals = mintInfo.decimals;
            }
        } catch {
            // No SPL interface exists
        }
    }

    if (wrap) {
        // UNIFIED MODE: Everything goes to c-token ATA

        // 1. Create c-token ATA if needed
        if (!ctokenHotSource) {
            instructions.push(
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer,
                    ctokenAtaAddress,
                    owner,
                    mint,
                    CTOKEN_PROGRAM_ID,
                ),
            );
        }

        // 2. Wrap SPL tokens to c-token
        if (splBalance > BigInt(0) && splInterfaceInfo) {
            instructions.push(
                createWrapInstruction(
                    splAta,
                    ctokenAtaAddress,
                    owner,
                    mint,
                    splBalance,
                    splInterfaceInfo,
                    decimals,
                    payer,
                ),
            );
        }

        // 3. Wrap T22 tokens to c-token
        if (t22Balance > BigInt(0) && splInterfaceInfo) {
            instructions.push(
                createWrapInstruction(
                    t22Ata,
                    ctokenAtaAddress,
                    owner,
                    mint,
                    t22Balance,
                    splInterfaceInfo,
                    decimals,
                    payer,
                ),
            );
        }

        // 4. Decompress compressed tokens to c-token ATA
        // Note: v3 interface only supports V2 trees
        if (coldBalance > BigInt(0) && ctokenColdSource) {
            const compressedAccounts =
                getCompressedTokenAccountsFromAtaSources(sources);

            if (compressedAccounts.length > 0) {
                assertV2Only(compressedAccounts);

                const proof = await rpc.getValidityProofV0(
                    compressedAccounts.map(acc => ({
                        hash: acc.compressedAccount.hash,
                        tree: acc.compressedAccount.treeInfo.tree,
                        queue: acc.compressedAccount.treeInfo.queue,
                    })),
                );

                instructions.push(
                    createDecompressInterfaceInstruction(
                        payer,
                        compressedAccounts,
                        ctokenAtaAddress,
                        coldBalance,
                        proof,
                        undefined,
                        decimals,
                    ),
                );
            }
        }
    } else {
        // STANDARD MODE: Decompress to target ATA type

        if (coldBalance > BigInt(0) && ctokenColdSource) {
            const compressedAccounts =
                getCompressedTokenAccountsFromAtaSources(sources);

            if (compressedAccounts.length > 0) {
                assertV2Only(compressedAccounts);

                const proof = await rpc.getValidityProofV0(
                    compressedAccounts.map(acc => ({
                        hash: acc.compressedAccount.hash,
                        tree: acc.compressedAccount.treeInfo.tree,
                        queue: acc.compressedAccount.treeInfo.queue,
                    })),
                );

                if (ataType === 'ctoken') {
                    // Decompress to c-token ATA (direct)
                    if (!ctokenHotSource) {
                        instructions.push(
                            createAssociatedTokenAccountInterfaceIdempotentInstruction(
                                payer,
                                ctokenAtaAddress,
                                owner,
                                mint,
                                CTOKEN_PROGRAM_ID,
                            ),
                        );
                    }
                    instructions.push(
                        createDecompressInterfaceInstruction(
                            payer,
                            compressedAccounts,
                            ctokenAtaAddress,
                            coldBalance,
                            proof,
                            undefined,
                            decimals,
                        ),
                    );
                } else if (ataType === 'spl' && splInterfaceInfo) {
                    // Decompress to SPL ATA via token pool
                    // Create SPL ATA if needed
                    if (!splSource) {
                        instructions.push(
                            createAssociatedTokenAccountIdempotentInstruction(
                                payer,
                                splAta,
                                owner,
                                mint,
                                TOKEN_PROGRAM_ID,
                            ),
                        );
                    }
                    instructions.push(
                        createDecompressInterfaceInstruction(
                            payer,
                            compressedAccounts,
                            splAta,
                            coldBalance,
                            proof,
                            splInterfaceInfo,
                            decimals,
                        ),
                    );
                } else if (ataType === 'token2022' && splInterfaceInfo) {
                    // Decompress to T22 ATA via token pool
                    // Create T22 ATA if needed
                    if (!t22Source) {
                        instructions.push(
                            createAssociatedTokenAccountIdempotentInstruction(
                                payer,
                                t22Ata,
                                owner,
                                mint,
                                TOKEN_2022_PROGRAM_ID,
                            ),
                        );
                    }
                    instructions.push(
                        createDecompressInterfaceInstruction(
                            payer,
                            compressedAccounts,
                            t22Ata,
                            coldBalance,
                            proof,
                            splInterfaceInfo,
                            decimals,
                        ),
                    );
                }
            }
        }
    }

    return instructions;
}

/**
 * Load token balances into an ATA.
 *
 * Behavior depends on `wrap` parameter:
 * - wrap=false (standard): Decompress compressed tokens to the target ATA.
 *   ATA can be SPL (via pool), T22 (via pool), or c-token (direct).
 * - wrap=true (unified): Wrap SPL/T22 + decompress all to c-token ATA.
 *
 * Idempotent: returns null if nothing to load.
 *
 * @param rpc               RPC connection
 * @param ata               Associated token address (SPL, T22, or c-token)
 * @param owner             Owner of the tokens (signer)
 * @param mint              Mint public key
 * @param payer             Fee payer (signer, defaults to owner)
 * @param confirmOptions    Optional confirm options
 * @param interfaceOptions  Optional interface options
 * @param wrap              Unified mode: wrap SPL/T22 to c-token (default: false)
 * @returns Transaction signature, or null if nothing to load
 */
export async function loadAta(
    rpc: Rpc,
    ata: PublicKey,
    owner: Signer,
    mint: PublicKey,
    payer?: Signer,
    confirmOptions?: ConfirmOptions,
    interfaceOptions?: InterfaceOptions,
    wrap = false,
): Promise<TransactionSignature | null> {
    payer ??= owner;

    const ixs = await createLoadAtaInstructions(
        rpc,
        ata,
        owner.publicKey,
        mint,
        payer.publicKey,
        interfaceOptions,
        wrap,
    );

    if (ixs.length === 0) {
        return null;
    }

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ...ixs],
        payer,
        blockhash,
        additionalSigners,
    );

    return sendAndConfirmTx(rpc, tx, confirmOptions);
}
