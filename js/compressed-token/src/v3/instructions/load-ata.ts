import {
    Rpc,
    LIGHT_TOKEN_PROGRAM_ID,
    ParsedTokenAccount,
    bn,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import { assertV2Only } from '../assert-v2-only';
import {
    ComputeBudgetProgram,
    PublicKey,
    TransactionInstruction,
} from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
    createAssociatedTokenAccountIdempotentInstruction,
    TokenAccountNotFoundError,
} from '@solana/spl-token';
import {
    AccountInterface,
    checkNotFrozen,
    COLD_SOURCE_TYPES,
    getAtaInterface as _getAtaInterface,
    TokenAccountSource,
    isAuthorityForInterface,
    filterInterfaceForAuthority,
} from '../get-account-interface';
import { getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from './create-ata-interface';
import { createWrapInstruction } from './wrap';
import { createDecompressInterfaceInstruction } from './create-decompress-interface-instruction';
import {
    getSplInterfaceInfos,
    SplInterfaceInfo,
} from '../../utils/get-token-pool-infos';
import { getAtaProgramId, checkAtaAddress, AtaType } from '../ata-utils';
import type { InterfaceOptions } from '../actions/transfer-interface';

export const MAX_INPUT_ACCOUNTS = 8;

function chunkArray<T>(array: T[], chunkSize: number): T[][] {
    const chunks: T[][] = [];
    for (let i = 0; i < array.length; i += chunkSize) {
        chunks.push(array.slice(i, i + chunkSize));
    }
    return chunks;
}

export function selectInputsForAmount(
    accounts: ParsedTokenAccount[],
    neededAmount: bigint,
): ParsedTokenAccount[] {
    if (accounts.length === 0 || neededAmount <= BigInt(0)) return [];

    const sorted = [...accounts].sort((a, b) => {
        const amtA = BigInt(a.parsed.amount.toString());
        const amtB = BigInt(b.parsed.amount.toString());
        if (amtB > amtA) return 1;
        if (amtB < amtA) return -1;
        return 0;
    });

    let accumulated = BigInt(0);
    let countNeeded = 0;
    for (const acc of sorted) {
        countNeeded++;
        accumulated += BigInt(acc.parsed.amount.toString());
        if (accumulated >= neededAmount) break;
    }

    const selectCount = Math.min(
        Math.max(countNeeded, MAX_INPUT_ACCOUNTS),
        sorted.length,
    );

    return sorted.slice(0, selectCount);
}

function assertUniqueInputHashes(chunks: ParsedTokenAccount[][]): void {
    const seen = new Set<string>();
    for (const chunk of chunks) {
        for (const acc of chunk) {
            const hashStr = acc.compressedAccount.hash.toString();
            if (seen.has(hashStr)) {
                throw new Error(
                    `Duplicate compressed account hash across chunks: ${hashStr}. ` +
                        `Each compressed account must appear in exactly one chunk.`,
                );
            }
            seen.add(hashStr);
        }
    }
}

export function getCompressedTokenAccountsFromAtaSources(
    sources: TokenAccountSource[],
): ParsedTokenAccount[] {
    return sources
        .filter(source => source.loadContext !== undefined)
        .filter(source => COLD_SOURCE_TYPES.has(source.type))
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

export { AtaType } from '../ata-utils';

export async function createLoadAtaInstructions(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    decimals: number,
    payer?: PublicKey,
    interfaceOptions?: InterfaceOptions,
): Promise<TransactionInstruction[][]> {
    assertBetaEnabled();
    payer ??= owner;
    const wrap = interfaceOptions?.wrap ?? false;

    const effectiveOwner = interfaceOptions?.owner ?? owner;

    let accountInterface: AccountInterface;
    try {
        accountInterface = await _getAtaInterface(
            rpc,
            ata,
            effectiveOwner,
            mint,
            undefined,
            undefined,
            wrap,
        );
    } catch (e) {
        if (e instanceof TokenAccountNotFoundError) {
            return [];
        }
        throw e;
    }

    const isDelegate = !effectiveOwner.equals(owner);
    if (isDelegate) {
        if (!isAuthorityForInterface(accountInterface, owner)) {
            throw new Error(
                'Signer is not the owner or a delegate of the account.',
            );
        }
        accountInterface = filterInterfaceForAuthority(accountInterface, owner);
    }

    const internalBatches = await _buildLoadBatches(
        rpc,
        payer,
        accountInterface,
        interfaceOptions,
        wrap,
        ata,
        undefined,
        owner,
        decimals,
    );

    return internalBatches.map(batch => [
        ComputeBudgetProgram.setComputeUnitLimit({
            units: calculateLoadBatchComputeUnits(batch),
        }),
        ...batch.instructions,
    ]);
}

export interface InternalLoadBatch {
    instructions: TransactionInstruction[];
    compressedAccounts: ParsedTokenAccount[];
    wrapCount: number;
    hasAtaCreation: boolean;
}

const CU_ATA_CREATION = 30_000;
const CU_WRAP = 50_000;
const CU_DECOMPRESS_BASE = 50_000;
const CU_FULL_PROOF = 100_000;
const CU_PER_ACCOUNT_PROVE_BY_INDEX = 10_000;
const CU_PER_ACCOUNT_FULL_PROOF = 30_000;
const CU_BUFFER_FACTOR = 1.3;
const CU_MIN = 50_000;
const CU_MAX = 1_400_000;

export function rawLoadBatchComputeUnits(batch: InternalLoadBatch): number {
    let cu = 0;
    if (batch.hasAtaCreation) cu += CU_ATA_CREATION;
    cu += batch.wrapCount * CU_WRAP;
    if (batch.compressedAccounts.length > 0) {
        cu += CU_DECOMPRESS_BASE;
        const needsFullProof = batch.compressedAccounts.some(
            acc => !(acc.compressedAccount.proveByIndex ?? false),
        );
        if (needsFullProof) cu += CU_FULL_PROOF;
        for (const acc of batch.compressedAccounts) {
            cu +=
                (acc.compressedAccount.proveByIndex ?? false)
                    ? CU_PER_ACCOUNT_PROVE_BY_INDEX
                    : CU_PER_ACCOUNT_FULL_PROOF;
        }
    }
    return cu;
}

export function calculateLoadBatchComputeUnits(
    batch: InternalLoadBatch,
): number {
    const cu = Math.ceil(rawLoadBatchComputeUnits(batch) * CU_BUFFER_FACTOR);
    return Math.max(CU_MIN, Math.min(CU_MAX, cu));
}

export async function _buildLoadBatches(
    rpc: Rpc,
    payer: PublicKey,
    ata: AccountInterface,
    options: InterfaceOptions | undefined,
    wrap: boolean,
    targetAta: PublicKey,
    targetAmount: bigint | undefined,
    authority: PublicKey | undefined,
    decimals: number,
): Promise<InternalLoadBatch[]> {
    if (!ata._isAta || !ata._owner || !ata._mint) {
        throw new Error(
            'AccountInterface must be from getAtaInterface (requires _isAta, _owner, _mint)',
        );
    }

    checkNotFrozen(ata, 'load');

    const owner = ata._owner;
    const mint = ata._mint;
    const sources = ata._sources ?? [];

    const allCompressedAccounts =
        getCompressedTokenAccountsFromAtaSources(sources);

    const lightTokenAtaAddress = getAssociatedTokenAddressInterface(
        mint,
        owner,
    );
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

    let ataType: AtaType = 'light-token';
    const validation = checkAtaAddress(targetAta, mint, owner);
    ataType = validation.type;
    if (wrap && ataType !== 'light-token') {
        throw new Error(
            `For wrap=true, targetAta must be light-token associated token account. Got ${ataType} associated token account.`,
        );
    }

    const splSource = sources.find(s => s.type === 'spl');
    const t22Source = sources.find(s => s.type === 'token2022');
    const lightTokenHotSource = sources.find(s => s.type === 'light-token-hot');
    const coldSources = sources.filter(s => COLD_SOURCE_TYPES.has(s.type));

    const splBalance = splSource?.amount ?? BigInt(0);
    const t22Balance = t22Source?.amount ?? BigInt(0);
    const coldBalance = coldSources.reduce(
        (sum, s) => sum + s.amount,
        BigInt(0),
    );

    if (
        splBalance === BigInt(0) &&
        t22Balance === BigInt(0) &&
        coldBalance === BigInt(0)
    ) {
        return [];
    }

    let splInterfaceInfo: SplInterfaceInfo | undefined;
    const needsSplInfo =
        wrap ||
        ataType === 'spl' ||
        ataType === 'token2022' ||
        splBalance > BigInt(0) ||
        t22Balance > BigInt(0);
    if (needsSplInfo) {
        try {
            const splInterfaceInfos =
                options?.splInterfaceInfos ??
                (await getSplInterfaceInfos(rpc, mint));
            splInterfaceInfo = splInterfaceInfos.find(
                (info: SplInterfaceInfo) => info.isInitialized,
            );
        } catch (e) {
            if (splBalance > BigInt(0) || t22Balance > BigInt(0)) {
                throw e;
            }
        }
    }

    const setupInstructions: TransactionInstruction[] = [];
    let wrapCount = 0;
    let needsAtaCreation = false;

    let decompressTarget: PublicKey = lightTokenAtaAddress;
    let decompressSplInfo: SplInterfaceInfo | undefined;
    let canDecompress = false;

    if (wrap) {
        decompressTarget = lightTokenAtaAddress;
        decompressSplInfo = undefined;
        canDecompress = true;

        if (!lightTokenHotSource) {
            needsAtaCreation = true;
            setupInstructions.push(
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer,
                    lightTokenAtaAddress,
                    owner,
                    mint,
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
            );
        }

        if (splBalance > BigInt(0) && splInterfaceInfo) {
            setupInstructions.push(
                createWrapInstruction(
                    splAta,
                    lightTokenAtaAddress,
                    owner,
                    mint,
                    splBalance,
                    splInterfaceInfo,
                    decimals,
                    payer,
                ),
            );
            wrapCount++;
        }

        if (t22Balance > BigInt(0) && splInterfaceInfo) {
            setupInstructions.push(
                createWrapInstruction(
                    t22Ata,
                    lightTokenAtaAddress,
                    owner,
                    mint,
                    t22Balance,
                    splInterfaceInfo,
                    decimals,
                    payer,
                ),
            );
            wrapCount++;
        }
    } else {
        if (ataType === 'light-token') {
            decompressTarget = lightTokenAtaAddress;
            decompressSplInfo = undefined;
            canDecompress = true;
            if (!lightTokenHotSource) {
                needsAtaCreation = true;
                setupInstructions.push(
                    createAssociatedTokenAccountInterfaceIdempotentInstruction(
                        payer,
                        lightTokenAtaAddress,
                        owner,
                        mint,
                        LIGHT_TOKEN_PROGRAM_ID,
                    ),
                );
            }
        } else if (ataType === 'spl' && splInterfaceInfo) {
            decompressTarget = splAta;
            decompressSplInfo = splInterfaceInfo;
            canDecompress = true;
            if (!splSource) {
                needsAtaCreation = true;
                setupInstructions.push(
                    createAssociatedTokenAccountIdempotentInstruction(
                        payer,
                        splAta,
                        owner,
                        mint,
                        TOKEN_PROGRAM_ID,
                    ),
                );
            }
        } else if (ataType === 'token2022' && splInterfaceInfo) {
            decompressTarget = t22Ata;
            decompressSplInfo = splInterfaceInfo;
            canDecompress = true;
            if (!t22Source) {
                needsAtaCreation = true;
                setupInstructions.push(
                    createAssociatedTokenAccountIdempotentInstruction(
                        payer,
                        t22Ata,
                        owner,
                        mint,
                        TOKEN_2022_PROGRAM_ID,
                    ),
                );
            }
        }
    }

    let accountsToLoad = allCompressedAccounts;

    if (
        targetAmount !== undefined &&
        canDecompress &&
        allCompressedAccounts.length > 0
    ) {
        const isDelegate = authority !== undefined && !authority.equals(owner);
        const hotBalance = (() => {
            if (!lightTokenHotSource) return BigInt(0);
            if (isDelegate) {
                const delegated =
                    lightTokenHotSource.parsed.delegatedAmount ?? BigInt(0);
                return delegated < lightTokenHotSource.amount
                    ? delegated
                    : lightTokenHotSource.amount;
            }
            return lightTokenHotSource.amount;
        })();
        let effectiveHotAfterSetup: bigint;

        if (wrap) {
            effectiveHotAfterSetup = hotBalance + splBalance + t22Balance;
        } else if (ataType === 'light-token') {
            effectiveHotAfterSetup = hotBalance;
        } else if (ataType === 'spl') {
            effectiveHotAfterSetup = splBalance;
        } else {
            effectiveHotAfterSetup = t22Balance;
        }

        const neededFromCold =
            targetAmount > effectiveHotAfterSetup
                ? targetAmount - effectiveHotAfterSetup
                : BigInt(0);

        if (neededFromCold === BigInt(0)) {
            accountsToLoad = [];
        } else {
            accountsToLoad = selectInputsForAmount(
                allCompressedAccounts,
                neededFromCold,
            );
        }
    }

    if (!canDecompress || accountsToLoad.length === 0) {
        if (setupInstructions.length === 0) return [];
        return [
            {
                instructions: setupInstructions,
                compressedAccounts: [],
                wrapCount,
                hasAtaCreation: needsAtaCreation,
            },
        ];
    }

    assertV2Only(accountsToLoad);

    const chunks = chunkArray(accountsToLoad, MAX_INPUT_ACCOUNTS);
    assertUniqueInputHashes(chunks);

    const proofs = await Promise.all(
        chunks.map(async chunk => {
            const proofInputs = chunk.map(acc => ({
                hash: acc.compressedAccount.hash,
                tree: acc.compressedAccount.treeInfo.tree,
                queue: acc.compressedAccount.treeInfo.queue,
            }));
            return rpc.getValidityProofV0(proofInputs);
        }),
    );

    const idempotentAtaIx = (() => {
        if (wrap || ataType === 'light-token') {
            return createAssociatedTokenAccountInterfaceIdempotentInstruction(
                payer,
                lightTokenAtaAddress,
                owner,
                mint,
                LIGHT_TOKEN_PROGRAM_ID,
            );
        } else if (ataType === 'spl') {
            return createAssociatedTokenAccountIdempotentInstruction(
                payer,
                splAta,
                owner,
                mint,
                TOKEN_PROGRAM_ID,
            );
        } else {
            return createAssociatedTokenAccountIdempotentInstruction(
                payer,
                t22Ata,
                owner,
                mint,
                TOKEN_2022_PROGRAM_ID,
            );
        }
    })();

    const batches: InternalLoadBatch[] = [];

    for (let i = 0; i < chunks.length; i++) {
        const chunk = chunks[i];
        const proof = proofs[i];
        const chunkAmount = chunk.reduce(
            (sum, acc) => sum + BigInt(acc.parsed.amount.toString()),
            BigInt(0),
        );

        const batchIxs: TransactionInstruction[] = [];
        let batchWrapCount = 0;
        let batchHasAtaCreation = false;

        if (i === 0) {
            batchIxs.push(...setupInstructions);
            batchWrapCount = wrapCount;
            batchHasAtaCreation = needsAtaCreation;
        } else {
            batchIxs.push(idempotentAtaIx);
            batchHasAtaCreation = true;
        }

        const authorityForDecompress = authority ?? owner;
        batchIxs.push(
            createDecompressInterfaceInstruction(
                payer,
                chunk,
                decompressTarget,
                chunkAmount,
                proof,
                decompressSplInfo,
                decimals,
                undefined,
                authorityForDecompress,
            ),
        );

        batches.push({
            instructions: batchIxs,
            compressedAccounts: chunk,
            wrapCount: batchWrapCount,
            hasAtaCreation: batchHasAtaCreation,
        });
    }

    return batches;
}
