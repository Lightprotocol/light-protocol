import { PublicKey } from '@solana/web3.js';
import {
    TokenData,
    CompressedTokenProgram,
} from '@lightprotocol/compressed-token';
import {
    CompressedAccountWithMerkleContext,
    CompressedAccount,
    PublicTransactionEvent,
    MerkleContext,
    createCompressedAccountWithMerkleContext,
    defaultTestStateTreeAccounts,
    Rpc,
    ParsedTokenAccount,
} from '@lightprotocol/stateless.js';
import { getParsedEvents } from './get-parsed-events';

export type EventWithParsedTokenTlvData = {
    inputCompressedAccounts: ParsedTokenAccount[];
    outputCompressedAccounts: ParsedTokenAccount[];
};

/** @internal */
function parseTokenLayoutWithIdl(
    compressedAccount: CompressedAccount,
): TokenData | null {
    if (compressedAccount.data === null) return null;

    const { data } = compressedAccount.data;

    if (data.length === 0) return null;
    if (
        compressedAccount.owner.toBase58() !==
        CompressedTokenProgram.programId.toBase58()
    ) {
        throw new Error(
            `Invalid owner ${compressedAccount.owner.toBase58()} for token layout`,
        );
    }
    const decodedLayout = CompressedTokenProgram.program.coder.types.decode(
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
    const inputHashes = event.inputCompressedAccountHashes;
    /// TODO: consider different structure
    const inputCompressedAccountWithParsedTokenData: ParsedTokenAccount[] =
        event.inputCompressedAccounts.map((compressedAccount, i) => {
            const merkleContext: MerkleContext = {
                merkleTree:
                    pubkeyArray[compressedAccount.merkleTreePubkeyIndex],
                nullifierQueue:
                    pubkeyArray[compressedAccount.nullifierQueuePubkeyIndex],
                hash: inputHashes[i],
                leafIndex: compressedAccount.leafIndex,
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

    const outputHashes = event.outputCompressedAccountHashes;
    const outputCompressedAccountsWithParsedTokenData: ParsedTokenAccount[] =
        event.outputCompressedAccounts.map((compressedAccount, i) => {
            const merkleContext: MerkleContext = {
                merkleTree:
                    pubkeyArray[event.outputStateMerkleTreeAccountIndices[i]],
                nullifierQueue:
                    // FIXME: fix make dynamic
                    defaultTestStateTreeAccounts().nullifierQueue,
                // pubkeyArray[event.outputStateMerkleTreeAccountIndices[i]],
                hash: outputHashes[i],
                leafIndex: event.outputLeafIndices[i],
            };

            if (!compressedAccount.data) throw new Error('No data');

            const parsedData = parseTokenLayoutWithIdl(compressedAccount);

            if (!parsedData) throw new Error('Invalid token data');

            const withMerkleContext = createCompressedAccountWithMerkleContext(
                merkleContext,
                compressedAccount.owner,
                compressedAccount.lamports,
                compressedAccount.data,
                compressedAccount.address ?? undefined,
            );
            return {
                compressedAccount: withMerkleContext,
                parsed: parsedData,
            };
        });

    return {
        inputCompressedAccounts: inputCompressedAccountWithParsedTokenData,
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
    const allInCompressedAccounts = eventsWithParsedTokenTlvData.flatMap(
        event => event.inputCompressedAccounts,
    );
    const unspentCompressedAccounts = allOutCompressedAccounts.filter(
        outputCompressedAccount =>
            !allInCompressedAccounts.some(
                inCompressedAccount =>
                    JSON.stringify(
                        inCompressedAccount.compressedAccount.hash,
                    ) ===
                    JSON.stringify(
                        outputCompressedAccount.compressedAccount.hash,
                    ),
            ),
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

    return compressedTokenAccounts.filter(
        acc => acc.parsed.owner.equals(owner) && acc.parsed.mint.equals(mint),
    );
}
