import { Connection, Keypair, PublicKey, Signer } from '@solana/web3.js';
import { confirmTx } from '../utils';
import { Rpc } from '../rpc';
import { LightWasm, WasmFactory } from '@lightprotocol/account.rs';
import { defaultTestStateTreeAccounts } from '../constants';
import { TestRpc } from './test-rpc';

let c = 1;

export const ALICE = getTestKeypair(255);
export const BOB = getTestKeypair(254);
export const CHARLIE = getTestKeypair(253);
export const DAVE = getTestKeypair(252);

export async function newAccountWithLamports(
  rpc: Rpc,
  lamports = 1000000000,
  counter: number | undefined = undefined,
): Promise<Signer> {
  const account = getTestKeypair(counter);
  const sig = await rpc.requestAirdrop(account.publicKey, lamports);
  await confirmTx(rpc, sig);
  return account;
}

export function getConnection(): Connection {
  const url = 'http://127.0.0.1:8899';
  const connection = new Connection(url, 'confirmed');
  return connection;
}

/**
 * Returns a mock RPC instance for use in unit tests.
 *
 * @param endpoint                RPC endpoint URL. Defaults to
 *                                'http://127.0.0.1:8899'.
 * @param proverEndpoint          Prover server endpoint URL. Defaults to
 *                                'http://localhost:3001'.
 * @param lightWasm               Wasm hasher instance.
 * @param merkleTreeAddress       Address of the merkle tree to index. Defaults
 *                                to the public default test state tree.
 * @param nullifierQueueAddress   Optional address of the associated nullifier
 *                                queue.
 * @param depth                   Depth of the merkle tree.
 * @param log                     Log proof generation time.
 */
export async function getTestRpc(
  endpoint = 'http://127.0.0.1:8899',
  proverEndpoint = 'http://localhost:3001',
  lightWasm?: LightWasm,
  merkleTreeAddress?: PublicKey,
  nullifierQueueAddress?: PublicKey,
  depth?: number,
  log = false,
) {
  lightWasm = lightWasm || (await WasmFactory.getInstance());

  const defaultAccounts = defaultTestStateTreeAccounts();

  return new TestRpc(
    endpoint,
    lightWasm,
    {
      merkleTreeAddress: merkleTreeAddress || defaultAccounts.merkleTree,
      nullifierQueueAddress:
        nullifierQueueAddress || defaultAccounts.nullifierQueue,
      depth: depth || defaultAccounts.merkleTreeHeight,
      log,
    },
    proverEndpoint,
  );
}

/**
 * For use in tests.
 * Generate a unique keypair by passing in a counter <255. If no counter
 * is supplied, it uses and increments a global counter.
 */
export function getTestKeypair(
  counter: number | undefined = undefined,
): Keypair {
  if (!counter) {
    counter = c;
    c++;
  }
  if (counter > 255) {
    throw new Error('Counter must be <= 255');
  }
  const seed = new Uint8Array(32);
  seed[0] = counter;

  return Keypair.fromSeed(seed);
}

//@ts-ignore
if (import.meta.vitest) {
  //@ts-ignore
  const { describe, it, expect } = import.meta.vitest;

  describe('getTestKeypair', () => {
    it('should generate a keypair with a specific counter', () => {
      const keypair = getTestKeypair(10);
      expect(keypair).toBeInstanceOf(Keypair);
      expect(keypair.publicKey).toBeDefined();
      expect(keypair.secretKey).toBeDefined();
    });

    it('should throw an error if counter is greater than 255', () => {
      const testFn = () => getTestKeypair(256);
      expect(testFn).toThrow('Counter must be <= 255');
    });

    it('should increment the global counter if no counter is provided', () => {
      const initialKeypair = getTestKeypair();
      const nextKeypair = getTestKeypair();
      expect(initialKeypair).not.toEqual(nextKeypair);
    });
  });
}
