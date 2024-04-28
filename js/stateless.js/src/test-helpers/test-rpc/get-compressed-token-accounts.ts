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
} from '../../state';

const tokenProgramId: PublicKey = new PublicKey(
    // TODO: can add check to ensure its consistent with the idl
    '9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE',
);

type TokenData = {
    mint: PublicKey;
    owner: PublicKey;
    amount: BN;
    delegate: PublicKey | null;
    state: number;
    isNative: BN | null;
    delegatedAmount: BN;
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
                    pubkeyArray[event.outputStateMerkleTreeAccountIndices[i]],
                nullifierQueue:
                    // FIXME: fix make dynamic
                    defaultTestStateTreeAccounts().nullifierQueue,
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

    return compressedTokenAccounts.filter(
        acc => acc.parsed.owner.equals(owner) && acc.parsed.mint.equals(mint),
    );
}
