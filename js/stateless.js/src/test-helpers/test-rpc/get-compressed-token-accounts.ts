import { PublicKey } from '@solana/web3.js';
import { getParsedEvents } from './get-parsed-events';
import BN from 'bn.js';
import { COMPRESSED_TOKEN_PROGRAM_ID, featureFlags } from '../../constants';
import { Rpc } from '../../rpc';
import { getStateTreeInfoByPubkey } from '../../utils/get-state-tree-infos';
import { ParsedTokenAccount, WithCursor } from '../../rpc-interface';
import {
    PublicTransactionEvent,
    MerkleContext,
    createCompressedAccountWithMerkleContextLegacy,
    bn,
    TreeType,
    CompressedAccountLegacy,
} from '../../state';
import {
    struct,
    publicKey,
    u64,
    option,
    vecU8,
    u8,
    Layout,
} from '@coral-xyz/borsh';

type TokenData = {
    mint: PublicKey;
    owner: PublicKey;
    amount: BN;
    delegate: PublicKey | null;
    state: number;
    tlv: Buffer | null;
};

// for test-rpc
export const TokenDataLayout: Layout<TokenData> = struct([
    publicKey('mint'),
    publicKey('owner'),
    u64('amount'),
    option(publicKey(), 'delegate'),
    u8('state'),
    option(vecU8(), 'tlv'),
]);

export type EventWithParsedTokenTlvData = {
    inputCompressedAccountHashes: number[][];
    outputCompressedAccounts: ParsedTokenAccount[];
};

/**
 * Manually parse the compressed token layout for a given compressed account.
 * @param compressedAccount - The compressed account
 * @returns The parsed token data
 */
export function parseTokenLayoutWithIdl(
    compressedAccount: CompressedAccountLegacy,
    programId: PublicKey = COMPRESSED_TOKEN_PROGRAM_ID,
): TokenData | null {
    if (compressedAccount.data === null) return null;

    const { data } = compressedAccount.data;

    if (data.length === 0) return null;

    if (compressedAccount.owner.toBase58() !== programId.toBase58()) {
        throw new Error(
            `Invalid owner ${compressedAccount.owner.toBase58()} for token layout`,
        );
    }
    try {
        const decoded = TokenDataLayout.decode(Buffer.from(data));
        return decoded;
    } catch (error) {
        console.error('Decoding error:', error);
        throw error;
    }
}

/**
 * parse compressed accounts of an event with token layout
 * @internal
 */
async function parseEventWithTokenTlvData(
    event: PublicTransactionEvent,
    rpc: Rpc,
): Promise<EventWithParsedTokenTlvData> {
    const pubkeyArray = event.pubkeyArray;
    const infos = await rpc.getStateTreeInfos();
    const outputHashes = event.outputCompressedAccountHashes;
    const outputCompressedAccountsWithParsedTokenData: ParsedTokenAccount[] =
        event.outputCompressedAccounts.map((compressedAccount, i) => {
            const maybeTree =
                pubkeyArray[event.outputCompressedAccounts[i].merkleTreeIndex];

            const treeInfo = getStateTreeInfoByPubkey(infos, maybeTree);

            if (
                !treeInfo.tree.equals(
                    pubkeyArray[
                        event.outputCompressedAccounts[i].merkleTreeIndex
                    ],
                ) &&
                (featureFlags.isV2()
                    ? !treeInfo.queue.equals(
                          pubkeyArray[
                              event.outputCompressedAccounts[i].merkleTreeIndex
                          ],
                      )
                    : true)
            ) {
                throw new Error('Invalid tree');
            }
            const merkleContext: MerkleContext = {
                treeInfo,
                hash: bn(outputHashes[i]),
                leafIndex: event.outputLeafIndices[i],
                // V2 trees are always proveByIndex in test-rpc.
                proveByIndex: treeInfo.treeType === TreeType.StateV2,
            };
            if (!compressedAccount.compressedAccount.data)
                throw new Error('No data');
            const parsedData = parseTokenLayoutWithIdl(
                compressedAccount.compressedAccount,
            );
            if (!parsedData) throw new Error('Invalid token data');
            const withMerkleContext =
                createCompressedAccountWithMerkleContextLegacy(
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
    rpc: Rpc,
): Promise<ParsedTokenAccount[]> {
    const eventsWithParsedTokenTlvData: EventWithParsedTokenTlvData[] =
        await Promise.all(
            events.map(event => parseEventWithTokenTlvData(event, rpc)),
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
                return bn(hash).eq(
                    outputCompressedAccount.compressedAccount.hash,
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
): Promise<WithCursor<ParsedTokenAccount[]>> {
    const events = await getParsedEvents(rpc);
    const compressedTokenAccounts = await getCompressedTokenAccounts(
        events,
        rpc,
    );
    const accounts = compressedTokenAccounts.filter(
        acc => acc.parsed.owner.equals(owner) && acc.parsed.mint.equals(mint),
    );
    return {
        items: accounts.sort(
            (a, b) =>
                a.compressedAccount.leafIndex - b.compressedAccount.leafIndex,
        ),
        cursor: null,
    };
}

export async function getCompressedTokenAccountsByDelegateTest(
    rpc: Rpc,
    delegate: PublicKey,
    mint: PublicKey,
): Promise<WithCursor<ParsedTokenAccount[]>> {
    const events = await getParsedEvents(rpc);

    const compressedTokenAccounts = await getCompressedTokenAccounts(
        events,
        rpc,
    );
    return {
        items: compressedTokenAccounts.filter(
            acc =>
                acc.parsed.delegate?.equals(delegate) &&
                acc.parsed.mint.equals(mint),
        ),
        cursor: null,
    };
}

export async function getCompressedTokenAccountByHashTest(
    rpc: Rpc,
    hash: BN,
): Promise<ParsedTokenAccount> {
    const events = await getParsedEvents(rpc);

    const compressedTokenAccounts = await getCompressedTokenAccounts(
        events,
        rpc,
    );

    const filtered = compressedTokenAccounts.filter(acc =>
        bn(acc.compressedAccount.hash).eq(hash),
    );
    if (filtered.length === 0) {
        throw new Error('No compressed account found');
    }
    return filtered[0];
}
