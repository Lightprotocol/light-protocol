import { Connection, PublicKey } from '@solana/web3.js';
import {
  Utxo_IdlType,
  getMockRpc,
  PublicTransactionEvent_IdlType,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from './program';
import { TokenTlvData_IdlType } from './types';

export function parseTokenLayoutWithIdl(
  utxo: Utxo_IdlType,
): TokenTlvData_IdlType | null {
  if (utxo.data === null) {
    return null;
  }

  if (utxo.data.tlvElements.length === 0) return null;
  if (
    /// We can assume 0th element is token
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
 *  TODO: upstream into mockrpc direclty and sync with rpc-interface
 * - gets parsed events
 * - for each event. oututxos
 */
export async function getCompressedTokenAccountsFromMockRpc(
  connection: Connection,
  owner: PublicKey,
  mint: PublicKey,
) {
  const rpc = await getMockRpc(connection);
  const publicTxEvents = await rpc.getParsedEvents();
}

export type ParsedCompressedTokenAccount = {
  utxo: Utxo_IdlType;
  parsed: TokenTlvData_IdlType;
};

/**
 * Parsed outUtxos of events by deserializer.
 * Deserializer should return TokenLayout type
 * @internal
 */
function parseCompressedTokenAccounts(
  events: PublicTransactionEvent_IdlType[],
): ParsedCompressedTokenAccount[] {
  const parsedCompressedTokenAccountResults: ParsedCompressedTokenAccount[] =
    events.flatMap((event) => {
      return event.outUtxos.flatMap((utxo) => {
        if (!utxo.data) return [];
        return {
          utxo: utxo,
          parsed: parseTokenLayoutWithIdl(utxo),
        };
      });
    });
  return parsedCompressedTokenAccountResults;
}
/**
 * Retrieve all compressed token accounts by owner
 *
 * Note that it always returns null for MerkleUpdateContexts
 *
 * @param owner Publickey of the owning user or program
 *
 * */
async function getCompressedTokenAccounts(
  owner: PublicKey,
  _config?: GetUtxoConfig,
): Promise<WithMerkleUpdateContext<UtxoWithMerkleContext>[]> {

  /// parsed all outUtxos from all events by token layout
  const compressedTokenAccounts: ParsedCompressedTokenAccount<any>[] =
    this.parseCompressedTokenAccounts(events);

  const matchingUtxos: UtxoWithMerkleContext[] = [];

  for (const cTokenAccount of compressedTokenAccounts) {
    const leafIndices = [...event.outUtxoIndices]; // Clone to prevent mutation
    for (const outUtxo of event.outUtxos) {
      const leafIndex = leafIndices.shift();
      if (!leafIndex) continue; // Safety check

      const utxoHashComputed = await createUtxoHash(
        this.lightWasm,
        outUtxo,
        this.merkleTreeAddress,
        leafIndex,
      );

      if (outUtxo.owner.equals(owner)) {
        const merkleContext = {
          merkleTree: this.merkleTreeAddress,
          nullifierQueue: this.nullifierQueueAddress,
          hash: utxoHashComputed,
          leafIndex: leafIndex,
        };
        const utxoWithMerkleContext = createUtxoWithMerkleContext(
          outUtxo.owner,
          outUtxo.lamports,
          outUtxo.data,
          merkleContext,
          outUtxo.address ?? undefined,
        );

        matchingUtxos.push(utxoWithMerkleContext);
      }
    }
  }

  // Note: MerkleUpdateContext is always null in this mock implementation
  return matchingUtxos.map((utxo) => ({ context: null, value: utxo }));
}
