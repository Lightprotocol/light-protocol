import { PublicKey } from '@solana/web3.js';
import {
    MerkleContext,
    defaultTestStateTreeAccounts,
    PublicTransactionEvent,
    CompressedAccount,
    CompressedAccountWithMerkleContext,
    createCompressedAccountWithMerkleContext,
    Rpc,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from './program';
import { TokenData } from './types';

export type CompressedAccountWithParsedTokenData = {
    compressedAccountWithMerkleContext: CompressedAccountWithMerkleContext;
    parsed: TokenData;
};

export type EventWithParsedTokenTlvData = {
    inputCompressedAccounts: CompressedAccountWithParsedTokenData[];
    outputCompressedAccounts: CompressedAccountWithParsedTokenData[];
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
    const inputCompressedAccountWithParsedTokenData: CompressedAccountWithParsedTokenData[] =
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
                compressedAccountWithMerkleContext: withMerkleContext,
                parsed: parsedData,
            };
        });

    const outputHashes = event.outputCompressedAccountHashes;
    const outputCompressedAccountsWithParsedTokenData: CompressedAccountWithParsedTokenData[] =
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
                compressedAccountWithMerkleContext: withMerkleContext,
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
    owner: PublicKey,
    mint: PublicKey,
): Promise<CompressedAccountWithParsedTokenData[]> {
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
                        inCompressedAccount.compressedAccountWithMerkleContext
                            .hash,
                    ) ===
                    JSON.stringify(
                        outputCompressedAccount
                            .compressedAccountWithMerkleContext.hash,
                    ),
            ),
    );

    /// apply filter (owner, mint)
    return unspentCompressedAccounts.filter(
        acc => acc.parsed.owner.equals(owner) && acc.parsed.mint.equals(mint),
    );
}

/** @internal */
export async function getCompressedTokenAccountsForTest(
    rpc: Rpc,
    refSender: PublicKey,
    refMint: PublicKey,
) {
    // @ts-ignore
    const events = await rpc.getParsedEvents();

    const compressedTokenAccounts = await getCompressedTokenAccounts(
        events,
        refSender,
        refMint,
    );
    return compressedTokenAccounts;
}
