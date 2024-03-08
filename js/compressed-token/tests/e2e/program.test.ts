import { describe, it, expect, beforeAll } from 'vitest';
import { CompressedTokenProgram } from '../../src/program';
import { SPL_TOKEN_MINT_RENT_EXEMPT_BALANCE } from '../../src/constants';
import {
  Connection,
  Keypair,
  TransactionMessage,
  VersionedTransaction,
  PublicKey,
} from '@solana/web3.js';
import {
  MockProof,
  bn,
  byteArrayToKeypair,
  confirmTx,
  defaultTestStateTreeAccounts,
  sendAndConfirmTx,
  UtxoWithBlinding,
} from '@lightprotocol/stateless.js';
import {
  TokenTransferOutUtxo,
  createTransferInstruction,
} from '../../src/instructions/transfer';
import { unpackMint, unpackAccount } from '@solana/spl-token';
import { BN } from '@coral-xyz/anchor';

/// Asserts that createMint() creates a new spl mint account + the respective system pool account
async function assertMintCreated(
  mint: PublicKey,
  authority: PublicKey,
  connection: Connection,
  decimals: number,
  poolAccount: PublicKey,
) {
  const mintAcc = await connection.getAccountInfo(mint);
  const unpackedMint = unpackMint(mint, mintAcc);

  // for mint account
  const mintAuthority = CompressedTokenProgram.deriveMintAuthorityPda(
    authority,
    mint,
  );

  expect(unpackedMint.mintAuthority?.toString()).toBe(mintAuthority.toString());
  expect(unpackedMint.supply).toBe(0n);
  expect(unpackedMint.decimals).toBe(decimals);
  expect(unpackedMint.isInitialized).toBe(true);
  expect(unpackedMint.freezeAuthority).toBe(null);
  expect(unpackedMint.tlvData.length).toBe(0);

  // for pool
  const poolAccountInfo = await connection.getAccountInfo(poolAccount);
  const unpackedPoolAccount = unpackAccount(poolAccount, poolAccountInfo);

  expect(unpackedPoolAccount.mint.toBase58()).toBe(mint.toBase58());
  expect(unpackedPoolAccount.amount).toBe(0n); // deal with bigint
  expect(unpackedPoolAccount.owner.toBase58()).toBe(mintAuthority.toBase58());
  expect(unpackedPoolAccount.delegate).toBe(null);
}

describe('Compressed Token Program test', () => {
  const keys = defaultTestStateTreeAccounts();
  const merkleTree = keys.merkleTree;
  const queue = keys.stateNullifierQueue;
  const payer = byteArrayToKeypair([
    122, 239, 192, 18, 21, 29, 237, 120, 104, 95, 247, 150, 181, 218, 207, 60,
    158, 110, 200, 246, 74, 226, 30, 223, 142, 138, 133, 194, 30, 254, 132, 236,
    227, 130, 162, 184, 215, 227, 81, 211, 134, 73, 118, 71, 219, 163, 243, 41,
    118, 21, 155, 87, 11, 53, 153, 130, 178, 126, 151, 86, 225, 36, 251, 130,
  ]);
  const bob = Keypair.generate();
  const connection = new Connection('http://localhost:8899', 'confirmed');
  const randomMint = Keypair.generate();
  const mintDecimals = 2;
  const charlie = Keypair.generate();
  const transferAmount = 5 * mintDecimals;

  beforeAll(async () => {
    const sig = await connection.requestAirdrop(payer.publicKey, 3e9);
    await confirmTx(connection, sig);
    const sig2 = await connection.requestAirdrop(bob.publicKey, 3e9);
    await confirmTx(connection, sig2);
  });

  it('should create mint', async () => {
    const rentExemptBalance = SPL_TOKEN_MINT_RENT_EXEMPT_BALANCE;

    const ixs = await CompressedTokenProgram.createMint({
      feePayer: payer.publicKey,
      mint: randomMint.publicKey,
      decimals: mintDecimals,
      authority: payer.publicKey,
      freezeAuthority: null,
      rentExemptBalance: rentExemptBalance,
    });

    /// Build and send Solana tx
    const { blockhash } = await connection.getLatestBlockhash();

    const messageV0 = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: blockhash,
      instructions: ixs,
    }).compileToV0Message();

    const tx = new VersionedTransaction(messageV0);
    tx.sign([payer, randomMint]);

    const txId = await sendAndConfirmTx(connection, tx);

    const poolAccount = CompressedTokenProgram.deriveTokenPoolPda(
      randomMint.publicKey,
    );
    await assertMintCreated(
      randomMint.publicKey,
      payer.publicKey,
      connection,
      mintDecimals,
      poolAccount,
    );
    console.log('created compressed Mint txId', txId);
  });

  it('should mint_to bob', async () => {
    const ix = await CompressedTokenProgram.mintTo({
      feePayer: payer.publicKey,
      mint: randomMint.publicKey,
      authority: payer.publicKey,
      amount: 100 * mintDecimals,
      toPubkey: bob.publicKey,
      merkleTree,
    });

    /// Build and send Solana tx
    const { blockhash } = await connection.getLatestBlockhash();
    const messageV0 = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: blockhash,
      instructions: [ix],
    }).compileToV0Message();
    const tx = new VersionedTransaction(messageV0);
    tx.sign([payer]);

    const txId = await sendAndConfirmTx(connection, tx);

    console.log(
      `minted ${
        1 * mintDecimals
      } tokens (mint: ${randomMint.publicKey.toBase58()}) to bob \n txId: ${txId}`,
    );
    /// TODO: assert output utxos after implementing proper beet serde
  });

  /// TODO: refactor
  type TokenTlvData = {
    mint: PublicKey;
    owner: PublicKey;
    amount: BN;
    delegate: PublicKey | null;
    state: number;
    isNative: null;
    delegatedAmount: BN;
  };

  type TlvDataElement = {
    discriminator: Uint8Array;
    owner: PublicKey;
    data: Uint8Array;
    dataHash: BN;
  };

  /// TODO(swen): still need to debug compressed token transfers produces inconsistent remaining accounts with what's expected on-chain.
  it.skip('should transfer n mint to charlie', async () => {
    const tlv: TokenTlvData = {
      mint: randomMint.publicKey,
      owner: bob.publicKey,
      amount: bn(1000 + transferAmount),
      delegate: null,
      state: 0x01, //'Initialized',
      isNative: null,
      delegatedAmount: bn(0),
    };

    const tlvData = CompressedTokenProgram.program.coder.types.encode(
      'TokenTlvData',
      tlv,
    );

    const tlvDataElement: TlvDataElement = {
      discriminator: Uint8Array.from({ length: 8 }, () => 2),
      owner: bob.publicKey,
      data: Uint8Array.from(tlvData),
      dataHash: bn(Uint8Array.from({ length: 32 }, () => 0)), // mock
    };

    const inUtxo: UtxoWithBlinding = {
      owner: bob.publicKey,
      blinding: Array.from({ length: 32 }, () => 0),
      lamports: 0,
      data: { tlvElements: [tlvDataElement] },
    };

    const changeUtxo: TokenTransferOutUtxo = {
      amount: bn(1000),
      owner: bob.publicKey,
      lamports: null,
      index_mt_account: 0,
    };

    let charlieOutUtxo: TokenTransferOutUtxo = {
      amount: bn(transferAmount),
      owner: charlie.publicKey,
      lamports: null,
      index_mt_account: 0,
    };

    const proof_mock: MockProof = {
      a: Array.from({ length: 32 }, () => 0),
      b: Array.from({ length: 64 }, () => 0),
      c: Array.from({ length: 32 }, () => 0),
    };

    const ix = await createTransferInstruction(
      payer.publicKey,
      bob.publicKey,
      [merkleTree],
      [queue],
      [merkleTree, merkleTree],
      [inUtxo],
      [charlieOutUtxo, changeUtxo],
      [0], // input state root indices
      proof_mock,
    );

    const ixs = [ix];
    const { blockhash } = await connection.getLatestBlockhash();
    const messageV0 = new TransactionMessage({
      payerKey: payer.publicKey,
      recentBlockhash: blockhash,
      instructions: ixs,
    }).compileToV0Message();
    const tx = new VersionedTransaction(messageV0);
    tx.sign([payer, bob]);

    await sendAndConfirmTx(connection, tx);
  });
});
