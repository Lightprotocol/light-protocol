import { PublicKey } from '@solana/web3.js';
import {
    Utxo_IdlType,
    PublicTransactionEvent_IdlType,
    MerkleContext,
    defaultTestStateTreeAccounts,
    createUtxoHash,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from './program';
import { TokenTlvData_IdlType } from './types';
import { WasmFactory } from '@lightprotocol/account.rs';
import { BN } from '@coral-xyz/anchor';
import { Rpc, TestRpc } from '@lightprotocol/stateless.js/src';

export type UtxoWithParsedTokenTlvData = {
    utxo: Utxo_IdlType;
    parsed: TokenTlvData_IdlType;
    merkleContext: MerkleContext | null;
};

export type EventWithParsedTokenTlvData = {
    inUtxos: UtxoWithParsedTokenTlvData[];
    outUtxos: UtxoWithParsedTokenTlvData[];
};

/** @internal */
function parseTokenLayoutWithIdl(
    utxo: Utxo_IdlType,
): TokenTlvData_IdlType | null {
    if (utxo.data === null) {
        return null;
    }

    if (utxo.data.tlvElements.length === 0) return null;
    if (
        /// TODO: adapt to support cPDA feature.
        /// We currently assume 0th element is token
        utxo.data.tlvElements[0].owner.toBase58() !==
        CompressedTokenProgram.programId.toBase58()
    ) {
        throw new Error(
            `Invalid owner ${utxo.data.tlvElements[0].owner.toBase58()} for token layout`,
        );
    }
    const tlvData = utxo.data.tlvElements[0].data;
    const decodedLayout = CompressedTokenProgram.program.coder.types.decode(
        'TokenTlvDataClient',
        Buffer.from(tlvData),
    );

    return decodedLayout;
}

/**
 * parse utxos of an event with token layout
 * @internal
 */
async function parseEventWithTokenTlvData(
    event: PublicTransactionEvent_IdlType,
): Promise<EventWithParsedTokenTlvData> {
    return {
        inUtxos: parseInUtxos(event.inUtxos),
        outUtxos: await parseOutUtxos(event.outUtxos, event.outUtxoIndices),
    };
}

/** @internal */
const parseInUtxos = (utxos: Utxo_IdlType[]) => {
    return utxos.reduce((acc, utxo) => {
        if (utxo.data) {
            const parsed = parseTokenLayoutWithIdl(utxo);
            if (parsed) acc.push({ utxo, parsed, merkleContext: null });
        }
        return acc;
    }, [] as UtxoWithParsedTokenTlvData[]);
};

/** @internal */
const parseOutUtxos = async (utxos: Utxo_IdlType[], outUtxoIndices: BN[]) => {
    const lightWasm = await WasmFactory.getInstance(); // TODO: pass

    const parsedUtxos: UtxoWithParsedTokenTlvData[] = [];
    for (const utxo of utxos) {
        if (utxo.data) {
            const parsed = parseTokenLayoutWithIdl(utxo);
            const leafIndex = outUtxoIndices.shift();
            if (leafIndex === undefined) {
                throw new Error(
                    'OutUtxoIndices must be same length as outUtxos',
                );
            }
            const { merkleTree, nullifierQueue } =
                defaultTestStateTreeAccounts(); // TODO: pass or read from event
            const utxoHash = await createUtxoHash(
                lightWasm,
                utxo,
                merkleTree,
                leafIndex,
            );
            const merkleContext: MerkleContext = {
                merkleTree,
                nullifierQueue,
                hash: utxoHash,
                leafIndex: leafIndex,
            };
            if (parsed) parsedUtxos.push({ utxo, parsed, merkleContext });
        }
    }
    return parsedUtxos;
};

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
    events: PublicTransactionEvent_IdlType[],
    owner: PublicKey,
    mint: PublicKey,
): Promise<UtxoWithParsedTokenTlvData[]> {
    const eventsWithParsedTokenTlvData = await Promise.all(
        events.map(event => parseEventWithTokenTlvData(event)),
    );

    /// strip spent utxos if an outUtxo of tx n is an inUtxo of tx n+m, it is
    /// spent
    const allOutUtxos = eventsWithParsedTokenTlvData.flatMap(
        event => event.outUtxos,
    );
    const allInUtxos = eventsWithParsedTokenTlvData.flatMap(
        event => event.inUtxos,
    );
    const unspentUtxos = allOutUtxos.filter(
        outUtxo =>
            !allInUtxos.some(
                inUtxo =>
                    JSON.stringify(inUtxo.utxo.blinding) ===
                    JSON.stringify(outUtxo.utxo.blinding),
            ),
    );

    /// apply filter (owner, mint)
    return unspentUtxos.filter(
        utxo =>
            utxo.parsed.owner.equals(owner) && utxo.parsed.mint.equals(mint),
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
