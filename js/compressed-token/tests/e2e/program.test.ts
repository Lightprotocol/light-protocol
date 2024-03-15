import { describe, it, expect, beforeAll, assert } from 'vitest';
import { CompressedTokenProgram } from '../../src/program';
import { SPL_TOKEN_MINT_RENT_EXEMPT_BALANCE } from '../../src/constants';
import {
  Connection,
  Keypair,
  TransactionMessage,
  VersionedTransaction,
  PublicKey,
  ComputeBudgetProgram,
} from '@solana/web3.js';
import {
  bn,
  byteArrayToKeypair,
  confirmTx,
  defaultTestStateTreeAccounts,
  sendAndConfirmTx,
  getMockRpc,
  Utxo_IdlType,
  TlvDataElement_IdlType,
  CompressedProof_IdlType,
  buildAndSignTx,
} from '@lightprotocol/stateless.js';
import { createTransferInstruction } from '../../src/instructions/transfer';
import { unpackMint, unpackAccount } from '@solana/spl-token';
import { BN } from '@coral-xyz/anchor';
import { createMint, mintTo, transfer } from '../../src/actions';
import {
  TokenTlvData_IdlType,
  TokenTransferOutUtxo_IdlType,
} from '../../src/types';

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

/// TODO: fix deserialization bug to enable assert for output utxos
describe('Compressed Token Program test', () => {
  const keys = defaultTestStateTreeAccounts();
  const merkleTree = keys.merkleTree;
  const queue = keys.nullifierQueue;
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

  it("should create mint using 'createMint' action function", async () => {
    const { mint, transactionSignature } = await createMint(
      connection,
      payer,
      payer.publicKey,
      mintDecimals,
      randomMint,
    );
    const poolAccount = CompressedTokenProgram.deriveTokenPoolPda(
      randomMint.publicKey,
    );
    assert(mint.equals(randomMint.publicKey));
    await assertMintCreated(
      randomMint.publicKey,
      payer.publicKey,
      connection,
      mintDecimals,
      poolAccount,
    );
    console.log('created compressed Mint txId', transactionSignature);
  });

  it('should mint_to bob using "mintTo" action function', async () => {
    const txId = await mintTo(
      connection,
      payer,
      randomMint.publicKey,
      bob.publicKey,
      payer.publicKey,
      100,
      [],
      merkleTree,
    );
    console.log(
      `minted ${100} tokens (mint: ${randomMint.publicKey.toBase58()}) to bob \n txId: ${txId}`,
    );
  });

  /// TODO: move these as unit tests to program.ts
  it.skip('should create mint', async () => {
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

  /// TODO: move these as unit tests to program.ts
  it.skip('should mint_to bob', async () => {
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

  it('should transfer using "transfer" action ', async () => {
    const txId = await transfer(
      connection,
      payer,
      randomMint.publicKey,
      100,
      bob,
      charlie.publicKey,
      merkleTree,
    );
    console.log(
      `bob (${bob.publicKey.toBase58()}) transferred ${100} tokens (mint: ${randomMint.publicKey.toBase58()}) to charlie (${charlie.publicKey.toBase58()}) \n txId: ${txId}`,
    );
  });

  it('should transfer n mint to charlie', async () => {
    const tlv: TokenTlvData_IdlType = {
      mint: randomMint.publicKey,
      owner: bob.publicKey,
      amount: bn(1000 + transferAmount),
      delegate: null,
      state: 1, //'Initialized',
      isNative: null,
      delegatedAmount: bn(0),
    };

    const tlvData = CompressedTokenProgram.program.coder.types.encode(
      'TokenTlvDataClient',
      tlv,
    );

    const tlvDataElement: TlvDataElement_IdlType = {
      discriminator: Array(8).fill(2),
      owner: bob.publicKey,
      data: Uint8Array.from(tlvData),
      dataHash: Array(32).fill(0), // mock
    };

    const inUtxo: Utxo_IdlType = {
      owner: bob.publicKey,
      blinding: Array.from({ length: 32 }, () => 0),
      lamports: new BN(0),
      data: { tlvElements: [tlvDataElement] },
      address: null,
    };

    const changeUtxo: TokenTransferOutUtxo_IdlType = {
      amount: bn(1000),
      owner: bob.publicKey,
      lamports: null,
      index_mt_account: 0,
    };

    const charlieOutUtxo: TokenTransferOutUtxo_IdlType = {
      amount: bn(transferAmount),
      owner: charlie.publicKey,
      lamports: null,
      index_mt_account: 0,
    };

    const proof_mock: CompressedProof_IdlType = {
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

    const ixs = [
      ComputeBudgetProgram.setComputeUnitLimit({ units: 1_400_000 }),
      ix,
    ];
    const { blockhash } = await connection.getLatestBlockhash();

    const signedTx = buildAndSignTx(ixs, payer, blockhash, [bob]);

    const txId = await sendAndConfirmTx(connection, signedTx);

    console.log(
      `bob (${bob.publicKey.toBase58()}) transferred ${transferAmount} tokens (mint: ${randomMint.publicKey.toBase58()}) to charlie (${charlie.publicKey.toBase58()}) \n txId: ${txId}`,
    );
    const mockRpc = await getMockRpc(connection);
    const indexedEvents = await mockRpc.getParsedEvents();
    assert.equal(indexedEvents.length, 3);
  });
});
