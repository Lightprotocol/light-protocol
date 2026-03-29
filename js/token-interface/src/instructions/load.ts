import {
    Rpc,
    LIGHT_TOKEN_PROGRAM_ID,
    assertV2Enabled,
} from '@lightprotocol/stateless.js';
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
    AccountView,
    checkNotFrozen,
    getAtaView as _getAtaView,
    isAuthorityForAccount,
    filterAccountForAuthority,
} from '../read/get-account';
import { getAssociatedTokenAddress } from '../read/associated-token-address';
import { createAtaIdempotent } from './ata';
import { createWrapInstruction } from './wrap';
import { getSplInterfaces, type SplInterface } from '../spl-interface';
import { getAtaProgramId, checkAtaAddress, AtaType } from '../read/ata-utils';
import type { LoadOptions } from '../load-options';
import { getMint } from '../read/get-mint';
import { toLoadOptions } from '../helpers';
import { getAtaAddress } from '../read';
import type { CreateLoadInstructionsInput } from '../types';
import { toInstructionPlan } from './_plan';
import { createDecompressInstruction } from './load/decompress';
import { selectPrimaryColdCompressedAccountForLoad } from './load/select-primary-cold-account';
export { createDecompressInstruction } from './load/decompress';

async function _buildLoadInstructions(
    rpc: Rpc,
    payer: PublicKey,
    ata: AccountView,
    options: LoadOptions | undefined,
    wrap: boolean,
    targetAta: PublicKey,
    targetAmount: bigint | undefined,
    authority: PublicKey | undefined,
    decimals: number,
    allowFrozen: boolean,
): Promise<TransactionInstruction[]> {
    if (!ata._isAta || !ata._owner || !ata._mint) {
        throw new Error(
            'AccountView must be from getAtaView (requires _isAta, _owner, _mint)',
        );
    }

    if (!allowFrozen) {
        checkNotFrozen(ata, 'load');
    }

    const owner = ata._owner;
    const mint = ata._mint;
    const sources = ata._sources ?? [];

    const primaryColdCompressedAccount =
        selectPrimaryColdCompressedAccountForLoad(sources);

    const lightTokenAtaAddress = getAssociatedTokenAddress(mint, owner);
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
    const splBalance = splSource?.amount ?? BigInt(0);
    const t22Balance = t22Source?.amount ?? BigInt(0);
    const coldBalance = primaryColdCompressedAccount
        ? BigInt(primaryColdCompressedAccount.parsed.amount.toString())
        : BigInt(0);

    if (
        splBalance === BigInt(0) &&
        t22Balance === BigInt(0) &&
        coldBalance === BigInt(0)
    ) {
        return [];
    }

    const needsSplProgram = ataType === 'spl' || splBalance > BigInt(0);
    const needsToken2022Program =
        ataType === 'token2022' || t22Balance > BigInt(0);

    let splProgramInterface: SplInterface | undefined;
    let token2022ProgramInterface: SplInterface | undefined;
    if (needsSplProgram || needsToken2022Program) {
        const splInterfaces =
            options?.splInterfaces ?? (await getSplInterfaces(rpc, mint));
        splProgramInterface = splInterfaces.find(
            info =>
                info.isInitialized &&
                info.tokenProgramId.equals(TOKEN_PROGRAM_ID),
        );
        token2022ProgramInterface = splInterfaces.find(
            info =>
                info.isInitialized &&
                info.tokenProgramId.equals(TOKEN_2022_PROGRAM_ID),
        );

        if (needsSplProgram && !splProgramInterface) {
            throw new Error(
                `No initialized SPL interface found for mint ${mint.toBase58()} and token program ${TOKEN_PROGRAM_ID.toBase58()}.`,
            );
        }
        if (needsToken2022Program && !token2022ProgramInterface) {
            throw new Error(
                `No initialized SPL interface found for mint ${mint.toBase58()} and token program ${TOKEN_2022_PROGRAM_ID.toBase58()}.`,
            );
        }
    }

    const setupInstructions: TransactionInstruction[] = [];

    let decompressTarget: PublicKey = lightTokenAtaAddress;
    let decompressSplInfo: SplInterface | undefined;
    let canDecompress = false;

    if (wrap) {
        decompressTarget = lightTokenAtaAddress;
        decompressSplInfo = undefined;
        canDecompress = true;

        if (!lightTokenHotSource) {
            setupInstructions.push(
                createAtaIdempotent({
                    payer,
                    associatedToken: lightTokenAtaAddress,
                    owner,
                    mint,
                    programId: LIGHT_TOKEN_PROGRAM_ID,
                }),
            );
        }

        if (splBalance > BigInt(0)) {
            setupInstructions.push(
                createWrapInstruction({
                    source: splAta,
                    destination: lightTokenAtaAddress,
                    owner,
                    mint,
                    amount: splBalance,
                    splInterface: splProgramInterface!,
                    decimals,
                    payer,
                }),
            );
        }

        if (t22Balance > BigInt(0)) {
            setupInstructions.push(
                createWrapInstruction({
                    source: t22Ata,
                    destination: lightTokenAtaAddress,
                    owner,
                    mint,
                    amount: t22Balance,
                    splInterface: token2022ProgramInterface!,
                    decimals,
                    payer,
                }),
            );
        }
    } else {
        if (ataType === 'light-token') {
            decompressTarget = lightTokenAtaAddress;
            decompressSplInfo = undefined;
            canDecompress = true;
            if (!lightTokenHotSource) {
                setupInstructions.push(
                    createAtaIdempotent({
                        payer,
                        associatedToken: lightTokenAtaAddress,
                        owner,
                        mint,
                        programId: LIGHT_TOKEN_PROGRAM_ID,
                    }),
                );
            }
        } else if (ataType === 'spl') {
            decompressTarget = splAta;
            decompressSplInfo = splProgramInterface!;
            canDecompress = true;
            if (!splSource) {
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
        } else if (ataType === 'token2022') {
            decompressTarget = t22Ata;
            decompressSplInfo = token2022ProgramInterface!;
            canDecompress = true;
            if (!t22Source) {
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

    let accountToLoad = primaryColdCompressedAccount;

    if (
        targetAmount !== undefined &&
        canDecompress &&
        primaryColdCompressedAccount
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
            accountToLoad = null;
        }
    }

    if (!canDecompress || !accountToLoad) {
        return setupInstructions;
    }

    const proof = await rpc.getValidityProofV0([
        {
            hash: accountToLoad.compressedAccount.hash,
            tree: accountToLoad.compressedAccount.treeInfo.tree,
            queue: accountToLoad.compressedAccount.treeInfo.queue,
        },
    ]);
    const authorityForDecompress = authority ?? owner;
    const amountToDecompress = BigInt(accountToLoad.parsed.amount.toString());

    return [
        ...setupInstructions,
        createDecompressInstruction({
            payer,
            inputCompressedTokenAccounts: [accountToLoad],
            toAddress: decompressTarget,
            amount: amountToDecompress,
            validityProof: proof,
            splInterface: decompressSplInfo,
            decimals,
            authority: authorityForDecompress,
        }),
    ];
}

export interface CreateLoadInstructionOptions
    extends CreateLoadInstructionsInput {
    authority?: PublicKey;
    wrap?: boolean;
    allowFrozen?: boolean;
    splInterfaces?: SplInterface[];
    decimals?: number;
}

function buildLoadOptions(
    owner: PublicKey,
    authority: PublicKey | undefined,
    wrap: boolean,
    splInterfaces: SplInterface[] | undefined,
): LoadOptions | undefined {
    const options = toLoadOptions(owner, authority, wrap) ?? {};
    if (splInterfaces) {
        options.splInterfaces = splInterfaces;
    }
    return Object.keys(options).length === 0 ? undefined : options;
}

export async function createLoadInstructions({
    rpc,
    payer,
    owner,
    mint,
    authority,
    wrap = true,
    allowFrozen = false,
    splInterfaces,
    decimals,
}: CreateLoadInstructionOptions): Promise<TransactionInstruction[]> {
    const targetAta = getAtaAddress({ owner, mint });
    const loadOptions = buildLoadOptions(owner, authority, wrap, splInterfaces);

    assertV2Enabled();
    payer ??= owner;
    const authorityPubkey = loadOptions?.delegatePubkey ?? owner;

    let accountView: AccountView;
    try {
        accountView = await _getAtaView(
            rpc,
            targetAta,
            owner,
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

    const resolvedDecimals =
        decimals ?? (await getMint(rpc, mint)).mint.decimals;

    if (!owner.equals(authorityPubkey)) {
        if (!isAuthorityForAccount(accountView, authorityPubkey)) {
            throw new Error(
                'Signer is not the owner or a delegate of the account.',
            );
        }
        accountView = filterAccountForAuthority(accountView, authorityPubkey);
    }

    const instructions = await _buildLoadInstructions(
        rpc,
        payer,
        accountView,
        loadOptions,
        wrap,
        targetAta,
        undefined,
        authorityPubkey,
        resolvedDecimals,
        allowFrozen,
    );

    if (instructions.length === 0) {
        return [];
    }
    return instructions.filter(
        instruction =>
            !instruction.programId.equals(ComputeBudgetProgram.programId),
    );
}

export async function createLoadInstructionPlan(
    input: CreateLoadInstructionsInput,
) {
    return toInstructionPlan(await createLoadInstructions(input));
}
