import { PublicKey } from '@solana/web3.js';
import { getParsedEvents } from './get-parsed-events';
import { BN, BorshCoder } from '@coral-xyz/anchor';

import { IDL } from '../../idls/light_compressed_token';
import { defaultTestStateTreeAccounts } from '../../constants';
import { Rpc } from '../../rpc';
import { ParsedTokenAccount } from '../../rpc-interface';
import {
    CompressedAccount,
    PublicTransactionEvent,
    MerkleContext,
    createCompressedAccountWithMerkleContext,
    bn,
} from '../../state';

const tokenProgramId: PublicKey = new PublicKey(
    // TODO: can add check to ensure its consistent with the idl
    'HXVfQ44ATEi9WBKLSCCwM54KokdkzqXci9xCQ7ST9SYN',
);

type TokenData = {
    mint: PublicKey;
    owner: PublicKey;
    amount: BN;
    delegate: PublicKey | null;
    state: number;
    tlv: Buffer | null;
};

export type EventWithParsedTokenTlvData = {
    inputCompressedAccountHashes: number[][];
    outputCompressedAccounts: ParsedTokenAccount[];
};

/** @internal */
function parseTokenLayoutWithIdl(
    compressedAccount: CompressedAccount,
): TokenData | null {
    if (compressedAccount.data === null) return null;

    const { data } = compressedAccount.data;

    if (data.length === 0) return null;
    if (compressedAccount.owner.toBase58() !== tokenProgramId.toBase58()) {
        throw new Error(
            `Invalid owner ${compressedAccount.owner.toBase58()} for token layout`,
        );
    }
    const decodedLayout = new BorshCoder(IDL).types.decode(
        'TokenData',
        Buffer.from(data),
    );

    return decodedLayout;
}

/**
 * parse compressed accounts of an event with token layout
 * @internal
 * TODO: refactor
 */
async function parseEventWithTokenTlvData(
    event: PublicTransactionEvent,
): Promise<EventWithParsedTokenTlvData> {
    const pubkeyArray = event.pubkeyArray;

    const outputHashes = event.outputCompressedAccountHashes;
    const outputCompressedAccountsWithParsedTokenData: ParsedTokenAccount[] =
        event.outputCompressedAccounts.map((compressedAccount, i) => {
            const merkleContext: MerkleContext = {
                merkleTree:
                    pubkeyArray[
                        event.outputCompressedAccounts[i].merkleTreeIndex
                    ],
                nullifierQueue:
                    // FIXME: fix make dynamic
                    defaultTestStateTreeAccounts().nullifierQueue,
                hash: outputHashes[i],
                leafIndex: event.outputLeafIndices[i],
            };

            if (!compressedAccount.compressedAccount.data)
                throw new Error('No data');

            const parsedData = parseTokenLayoutWithIdl(
                compressedAccount.compressedAccount,
            );

            if (!parsedData) throw new Error('Invalid token data');

            const withMerkleContext = createCompressedAccountWithMerkleContext(
                merkleContext,
                compressedAccount.compressedAccount.owner,
                compressedAccount.compressedAccount.lamports,
                compressedAccount.compressedAccount.data,
                compressedAccount.compressedAccount.address ?? undefined,
            );
            return {
                compressedAccount: withMerkleContext,
                parsed: parsedData,
            };
        });

    return {
        inputCompressedAccountHashes: event.inputCompressedAccountHashes,
        outputCompressedAccounts: outputCompressedAccountsWithParsedTokenData,
    };
}

/**
 * Retrieves all compressed token accounts for a given mint and owner.
 *
 * Note: This function is intended for testing purposes only. For production, use rpc.getCompressedTokenAccounts.
 *
 * @param events    Public transaction events
 * @param owner     PublicKey of the token owner
 * @param mint      PublicKey of the token mint
 */
export async function getCompressedTokenAccounts(
    events: PublicTransactionEvent[],
): Promise<ParsedTokenAccount[]> {
    const eventsWithParsedTokenTlvData: EventWithParsedTokenTlvData[] =
        await Promise.all(
            events.map(event => parseEventWithTokenTlvData(event)),
        );

    /// strip spent compressed accounts if an output compressed account of tx n is
    /// an input compressed account of tx n+m, it is spent
    const allOutCompressedAccounts = eventsWithParsedTokenTlvData.flatMap(
        event => event.outputCompressedAccounts,
    );
    const allInCompressedAccountHashes = eventsWithParsedTokenTlvData.flatMap(
        event => event.inputCompressedAccountHashes,
    );
    const unspentCompressedAccounts = allOutCompressedAccounts.filter(
        outputCompressedAccount =>
            !allInCompressedAccountHashes.some(hash => {
                return (
                    JSON.stringify(hash) ===
                    JSON.stringify(
                        outputCompressedAccount.compressedAccount.hash,
                    )
                );
            }),
    );

    return unspentCompressedAccounts;
}

/** @internal */
export async function getCompressedTokenAccountsByOwnerTest(
    rpc: Rpc,
    owner: PublicKey,
    mint: PublicKey,
): Promise<ParsedTokenAccount[]> {
    const events = await getParsedEvents(rpc);

    const compressedTokenAccounts = await getCompressedTokenAccounts(events);

    const accounts = compressedTokenAccounts.filter(
        acc => acc.parsed.owner.equals(owner) && acc.parsed.mint.equals(mint),
    );
    return accounts.sort(
        (a, b) => b.compressedAccount.leafIndex - a.compressedAccount.leafIndex,
    );
}

export async function getCompressedTokenAccountsByDelegateTest(
    rpc: Rpc,
    delegate: PublicKey,
    mint: PublicKey,
): Promise<ParsedTokenAccount[]> {
    const events = await getParsedEvents(rpc);

    const compressedTokenAccounts = await getCompressedTokenAccounts(events);

    return compressedTokenAccounts.filter(
        acc =>
            acc.parsed.delegate?.equals(delegate) &&
            acc.parsed.mint.equals(mint),
    );
}

export async function getCompressedTokenAccountByHashTest(
    rpc: Rpc,
    hash: BN,
): Promise<ParsedTokenAccount> {
    const events = await getParsedEvents(rpc);

    const compressedTokenAccounts = await getCompressedTokenAccounts(events);

    const filtered = compressedTokenAccounts.filter(acc =>
        bn(acc.compressedAccount.hash).eq(hash),
    );
    if (filtered.length === 0) {
        throw new Error('No compressed account found');
    }
    return filtered[0];
}
