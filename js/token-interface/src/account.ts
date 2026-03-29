import { getAssociatedTokenAddress } from './read/associated-token-address';
import { parseLightTokenCold, parseLightTokenHot } from './read/get-account';
import { Buffer } from 'buffer';
import {
    LIGHT_TOKEN_PROGRAM_ID,
    type ParsedTokenAccount,
    type Rpc,
} from '@lightprotocol/stateless.js';
import { TokenAccountNotFoundError } from '@solana/spl-token';
import type { PublicKey } from '@solana/web3.js';
import type {
    GetAtaInput,
    TokenInterfaceAccount,
    TokenInterfaceParsedAta,
} from './types';

const ZERO = BigInt(0);

function toBufferAccountInfo<T extends { data: Buffer | Uint8Array }>(
    accountInfo: T,
): Omit<T, 'data'> & { data: Buffer } {
    if (Buffer.isBuffer(accountInfo.data)) {
        return accountInfo as Omit<T, 'data'> & { data: Buffer };
    }
    return {
        ...accountInfo,
        data: Buffer.from(accountInfo.data),
    };
}

function toBigIntAmount(account: ParsedTokenAccount): bigint {
    return BigInt(account.parsed.amount.toString());
}

function sortCompressedAccounts(
    accounts: ParsedTokenAccount[],
): ParsedTokenAccount[] {
    return [...accounts].sort((left, right) => {
        const leftAmount = toBigIntAmount(left);
        const rightAmount = toBigIntAmount(right);

        if (rightAmount > leftAmount) {
            return 1;
        }

        if (rightAmount < leftAmount) {
            return -1;
        }

        return (
            right.compressedAccount.leafIndex - left.compressedAccount.leafIndex
        );
    });
}

function clampDelegatedAmount(amount: bigint, delegatedAmount: bigint): bigint {
    return delegatedAmount < amount ? delegatedAmount : amount;
}

function buildParsedAta(
    address: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    hotParsed: ReturnType<typeof parseLightTokenHot>['parsed'] | null,
    coldParsed: ReturnType<typeof parseLightTokenCold>['parsed'] | null,
): TokenInterfaceParsedAta {
    const hotAmount = hotParsed?.amount ?? ZERO;
    const compressedAmount = coldParsed?.amount ?? ZERO;
    const amount = hotAmount + compressedAmount;

    let delegate: PublicKey | null = null;
    let delegatedAmount = ZERO;

    if (hotParsed?.delegate) {
        delegate = hotParsed.delegate;
        delegatedAmount = hotParsed.delegatedAmount ?? ZERO;

        if (coldParsed?.delegate?.equals(delegate)) {
            delegatedAmount += clampDelegatedAmount(
                coldParsed.amount,
                coldParsed.delegatedAmount ?? coldParsed.amount,
            );
        }
    } else if (coldParsed?.delegate) {
        delegate = coldParsed.delegate;
        delegatedAmount = clampDelegatedAmount(
            coldParsed.amount,
            coldParsed.delegatedAmount ?? coldParsed.amount,
        );
    }

    return {
        address,
        owner,
        mint,
        amount,
        delegate,
        delegatedAmount: clampDelegatedAmount(amount, delegatedAmount),
        isInitialized: hotParsed?.isInitialized === true || coldParsed !== null,
        isFrozen: hotParsed?.isFrozen === true || coldParsed?.isFrozen === true,
    };
}

function selectPrimaryCompressedAccount(accounts: ParsedTokenAccount[]): {
    selected: ParsedTokenAccount | null;
    ignored: ParsedTokenAccount[];
} {
    const candidates = sortCompressedAccounts(
        accounts.filter(account => {
            return (
                account.compressedAccount.owner.equals(
                    LIGHT_TOKEN_PROGRAM_ID,
                ) &&
                account.compressedAccount.data !== null &&
                account.compressedAccount.data.data.length > 0 &&
                toBigIntAmount(account) > ZERO
            );
        }),
    );

    return {
        selected: candidates[0] ?? null,
        ignored: candidates.slice(1),
    };
}

export async function getAtaOrNull({
    rpc,
    owner,
    mint,
    commitment,
}: GetAtaInput): Promise<TokenInterfaceAccount | null> {
    const address = getAssociatedTokenAddress(mint, owner);

    const [hotInfo, compressedResult] = await Promise.all([
        rpc.getAccountInfo(address, commitment),
        rpc.getCompressedTokenAccountsByOwner(owner, { mint }),
    ]);

    const hotParsed =
        hotInfo && hotInfo.owner.equals(LIGHT_TOKEN_PROGRAM_ID)
            ? parseLightTokenHot(address, toBufferAccountInfo(hotInfo)).parsed
            : null;

    const { selected, ignored } = selectPrimaryCompressedAccount(
        compressedResult.items,
    );
    const coldParsed = selected
        ? parseLightTokenCold(address, selected.compressedAccount).parsed
        : null;

    if (!hotParsed && !coldParsed) {
        return null;
    }

    const parsed = buildParsedAta(address, owner, mint, hotParsed, coldParsed);
    const ignoredCompressedAmount = ignored.reduce(
        (sum, account) => sum + toBigIntAmount(account),
        ZERO,
    );

    return {
        address,
        owner,
        mint,
        amount: parsed.amount,
        hotAmount: hotParsed?.amount ?? ZERO,
        compressedAmount: coldParsed?.amount ?? ZERO,
        hasHotAccount: hotParsed !== null,
        requiresLoad: coldParsed !== null,
        parsed,
        compressedAccount: selected,
        ignoredCompressedAccounts: ignored,
        ignoredCompressedAmount,
    };
}

export async function getAta(
    input: GetAtaInput,
): Promise<TokenInterfaceAccount> {
    const account = await getAtaOrNull(input);

    if (!account) {
        throw new TokenAccountNotFoundError();
    }

    return account;
}

export function getSpendableAmount(
    account: TokenInterfaceAccount,
    authority: PublicKey,
): bigint {
    if (authority.equals(account.owner)) {
        return account.amount;
    }

    if (
        account.parsed.delegate !== null &&
        authority.equals(account.parsed.delegate)
    ) {
        return clampDelegatedAmount(
            account.amount,
            account.parsed.delegatedAmount,
        );
    }

    return ZERO;
}

export function assertAccountNotFrozen(
    account: TokenInterfaceAccount,
    operation: 'load' | 'transfer' | 'approve' | 'revoke' | 'burn' | 'freeze',
): void {
    if (account.parsed.isFrozen) {
        throw new Error(`Account is frozen; ${operation} is not allowed.`);
    }
}

export function assertAccountFrozen(
    account: TokenInterfaceAccount,
    operation: 'thaw',
): void {
    if (!account.parsed.isFrozen) {
        throw new Error(`Account is not frozen; ${operation} is not allowed.`);
    }
}
