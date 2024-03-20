import { BN } from '@coral-xyz/anchor';
import { Buffer } from 'buffer';
import { ConfirmOptions, PublicKey } from '@solana/web3.js';

export const FIELD_SIZE = new BN(
  '21888242871839275222246405745257275088548364400416034343698204186575808495617',
);

// TODO: implement properly
export const noopProgram = 'noopb9bkMVfRPU8AsbpTUg8AQkHtKwMYZiFUjNRtMmV';
export const lightProgram = '5WzvRtu7LABotw1SUEpguJiKU27LRGsiCnF5FH6VV7yP';
export const accountCompressionProgram = // also: merkletree program
  '5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN';

export const getRegisteredProgramPda = () =>
  new PublicKey('ytwwVWhQUMoTKdirKmvEW5xCRVr4B2dJZnToiHtE2L2'); // TODO: better labelling. gov authority pda

export const getPspAccountCompressionAuthority = () =>
  PublicKey.findProgramAddressSync(
    [
      Buffer.from('cpi_authority'),
      new PublicKey(accountCompressionProgram).toBytes(),
    ],
    new PublicKey(
      // TODO: can add check to ensure its consistent with the idl
      '6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ',
    ),
  )[0];

export const defaultStaticAccounts = () => [
  new PublicKey(getRegisteredProgramPda()),
  new PublicKey(noopProgram),
  new PublicKey(accountCompressionProgram),
  new PublicKey(getPspAccountCompressionAuthority()),
];
export const defaultStaticAccountsStruct = () => {
  return {
    registeredProgramPda: new PublicKey(getRegisteredProgramPda()),
    noopProgram: new PublicKey(noopProgram),
    accountCompressionProgram: new PublicKey(accountCompressionProgram),
    pspAccountCompressionAuthority: new PublicKey(
      getPspAccountCompressionAuthority(),
    ),
    cpiSignatureAccount: null,
  };
};

export const defaultTestStateTreeAccounts = () => {
  return {
    nullifierQueue: new PublicKey(nullifierQueuePubkey),
    merkleTree: new PublicKey(merkletreePubkey),
    merkleTreeHeight: DEFAULT_MERKLE_TREE_HEIGHT,
  };
};

export const nullifierQueuePubkey =
  '44J4oDXpjPAbzHCSc24q7NEiPekss4sAbLd8ka4gd9CZ'; // also called indexed_array

export const merkletreePubkey = '5bdFnXU47QjzGpzHfXnxcEi5WXyxzEAZzd1vrE39bf1W';

export const confirmConfig: ConfirmOptions = {
  commitment: 'confirmed',
  preflightCommitment: 'confirmed',
};

export const DEFAULT_MERKLE_TREE_HEIGHT = 26;
export const DEFAULT_MERKLE_TREE_ROOTS = 2800;
/** Threshold (per asset) at which new in-UTXOs get merged, in order to reduce UTXO pool size */
export const UTXO_MERGE_THRESHOLD = 20;
export const UTXO_MERGE_MAXIMUM = 10;

/**
 * Treshold after which the currently used transaction Merkle tree is switched
 * to the next one
 */
export const TRANSACTION_MERKLE_TREE_ROLLOVER_THRESHOLD = new BN(
  Math.floor(2 ** DEFAULT_MERKLE_TREE_HEIGHT * 0.95),
);
