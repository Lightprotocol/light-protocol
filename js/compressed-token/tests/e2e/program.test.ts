import { describe, it, expect, beforeAll, assert } from 'vitest';
import { CompressedTokenProgram } from '../../src/program';
import { SPL_TOKEN_MINT_RENT_EXEMPT_BALANCE } from '../../src/constants';
import {
  Connection,
  TransactionMessage,
  VersionedTransaction,
  PublicKey,
} from '@solana/web3.js';
import {
  bn,
  byteArrayToKeypair,
  confirmTx,
  defaultTestStateTreeAccounts,
  sendAndConfirmTx,
  getMockRpc,
} from '@lightprotocol/stateless.js';
import { unpackMint, unpackAccount } from '@solana/spl-token';
import { BN } from '@coral-xyz/anchor';
import { createMint, mintTo, transfer } from '../../src/actions';

import {
  UtxoWithParsedTokenTlvData,
  getCompressedTokenAccountsFromMockRpc,
} from '../../src/token-serde';

/**
 * Asserts that mintTo() creates a new compressed token account for the
 * recipient
 */
async function assertMintTo(
  connection: Connection,
  refMint: PublicKey,
  refAmount: BN,
  refTo: PublicKey,
) {
  const compressedTokenAccounts = await getCompressedTokenAccountsFromMockRpc(
    connection,
    refTo,
    refMint,
  );

  const compressedTokenAccount = compressedTokenAccounts[0];
  expect(compressedTokenAccount.parsed.mint.toBase58()).toBe(
    refMint.toBase58(),
  );
  expect(compressedTokenAccount.parsed.amount.eq(refAmount)).toBe(true);
  expect(compressedTokenAccount.parsed.owner.equals(refTo)).toBe(true);
  expect(compressedTokenAccount.parsed.delegate).toBe(null);
}

/**
 * Assert that we created recipient and change ctokens for the sender, with all
 * amounts correctly accounted for
 */
async function assertTransfer(
  connection: Connection,
  senderPreCompressedTokenAccounts: UtxoWithParsedTokenTlvData[], // all
  refMint: PublicKey,
  refAmount: BN,
  refSender: PublicKey,
  refRecipient: PublicKey,
  // TODO: add ...refValues
) {
  /// Transfer can merge input utxos therefore we need to pass all as ref
  const senderPostCompressedTokenAccounts =
    await getCompressedTokenAccountsFromMockRpc(connection, refSender, refMint);

  /// pre = post-amount
  const sumPre = senderPreCompressedTokenAccounts.reduce(
    (acc, curr) => bn(acc).add(curr.parsed.amount),
    bn(0),
  );
  const sumPost = senderPostCompressedTokenAccounts.reduce(
    (acc, curr) => bn(acc).add(curr.parsed.amount),
    bn(0),
  );

  expect(sumPre.sub(refAmount).eq(sumPost)).toBe(true);

  const recipientCompressedTokenAccounts =
    await getCompressedTokenAccountsFromMockRpc(
      connection,
      refRecipient,
      refMint,
    );

  /// recipient should have received the amount
  const recipientCompressedTokenAccount = recipientCompressedTokenAccounts[0];
  expect(recipientCompressedTokenAccount.parsed.amount.eq(refAmount)).toBe(
    true,
  );
  expect(recipientCompressedTokenAccount.parsed.delegate).toBe(null);
}

/**
 * Asserts that createMint() creates a new spl mint account + the respective
 * system pool account
 */
async function assertMintCreated(
  mint: PublicKey,
  authority: PublicKey,
  connection: Connection,
  decimals: number,
  poolAccount: PublicKey,
) {
  const mintAcc = await connection.getAccountInfo(mint);
  const unpackedMint = unpackMint(mint, mintAcc);

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

  /// Pool (omnibus) account is a regular SPL Token account
  const poolAccountInfo = await connection.getAccountInfo(poolAccount);
  const unpackedPoolAccount = unpackAccount(poolAccount, poolAccountInfo);
  expect(unpackedPoolAccount.mint.equals(mint)).toBe(true);
  expect(unpackedPoolAccount.amount).toBe(0n);
  expect(unpackedPoolAccount.owner.equals(mintAuthority)).toBe(true);
  expect(unpackedPoolAccount.delegate).toBe(null);
}

/// TODO: fix deserialization bug to enable assert for output utxos
describe('Compressed Token Program test', () => {
  const keys = defaultTestStateTreeAccounts();
  const merkleTree = keys.merkleTree;
  const payer = byteArrayToKeypair([
    122, 239, 192, 18, 21, 29, 237, 120, 104, 95, 247, 150, 181, 218, 207, 60,
    158, 110, 200, 246, 74, 226, 30, 223, 142, 138, 133, 194, 30, 254, 132, 236,
    227, 130, 162, 184, 215, 227, 81, 211, 134, 73, 118, 71, 219, 163, 243, 41,
    118, 21, 155, 87, 11, 53, 153, 130, 178, 126, 151, 86, 225, 36, 251, 130,
  ]);
  /// BNoVw1biCyJBkxGCtApddNCoTUa4UXy9TPDFgASbuvSk
  const bob = byteArrayToKeypair([
    254, 71, 194, 50, 63, 12, 211, 223, 250, 117, 253, 123, 220, 10, 100, 8,
    202, 254, 108, 65, 0, 153, 72, 157, 114, 212, 53, 112, 127, 206, 246, 153,
    154, 42, 164, 131, 114, 72, 61, 70, 40, 220, 171, 100, 231, 0, 42, 35, 249,
    7, 159, 126, 160, 250, 184, 187, 190, 120, 5, 31, 21, 130, 70, 233,
  ]);
  const connection = new Connection('http://localhost:8899', 'confirmed');
  /// DUgneYZ2Ly7w9yxUbbv9GZb9ngArc9qguoZYw2hCxwYw
  const fixedMint = byteArrayToKeypair([
    94, 171, 36, 11, 49, 160, 190, 169, 184, 238, 207, 210, 81, 218, 17, 112,
    149, 254, 184, 202, 86, 210, 205, 167, 80, 252, 195, 69, 178, 205, 96, 202,
    185, 99, 233, 139, 233, 54, 110, 239, 130, 16, 253, 78, 46, 210, 110, 241,
    63, 35, 100, 98, 171, 164, 116, 59, 163, 104, 7, 62, 220, 50, 192, 92,
  ]);

  const mintDecimals = 2;
  /// CUi16vFHptJ6qXCbfZyepE6nhZxpQYjafRQHJq7Rao27
  const charlie = byteArrayToKeypair([
    252, 121, 214, 211, 80, 145, 243, 18, 162, 210, 72, 174, 50, 106, 10, 171,
    216, 87, 101, 150, 28, 120, 246, 165, 148, 165, 63, 37, 187, 248, 141, 89,
    170, 137, 153, 79, 170, 147, 114, 99, 238, 127, 126, 118, 0, 157, 231, 171,
    213, 164, 122, 159, 81, 253, 145, 182, 136, 224, 236, 255, 0, 208, 123, 160,
  ]);

  beforeAll(async () => {
    const sig = await connection.requestAirdrop(payer.publicKey, 3e9);
    await confirmTx(connection, sig);
    const sig2 = await connection.requestAirdrop(bob.publicKey, 3e9);
    await confirmTx(connection, sig2);
  });

  it("should create mint using 'createMint' action function", async () => {
    const { mint } = await createMint(
      connection,
      payer,
      payer.publicKey,
      mintDecimals,
      fixedMint,
    );
    const poolAccount = CompressedTokenProgram.deriveTokenPoolPda(
      fixedMint.publicKey,
    );
    assert(mint.equals(fixedMint.publicKey));
    await assertMintCreated(
      fixedMint.publicKey,
      payer.publicKey,
      connection,
      mintDecimals,
      poolAccount,
    );
  });

  it('should mint_to bob using "mintTo" action function', async () => {
    await mintTo(
      connection,
      payer,
      fixedMint.publicKey,
      bob.publicKey,
      payer.publicKey,
      100, // 2 dec
      [],
      merkleTree,
    );

    await assertMintTo(connection, fixedMint.publicKey, bn(100), bob.publicKey);
  });

  it('should transfer using "transfer" action b->c, b->c, c->b, b<<', async () => {
    const bobPreCompressedTokenAccounts =
      await getCompressedTokenAccountsFromMockRpc(
        connection,
        bob.publicKey,
        fixedMint.publicKey,
      );

    await transfer(
      connection,
      payer,
      fixedMint.publicKey,
      70,
      bob,
      charlie.publicKey,
      merkleTree,
    );

    await assertTransfer(
      connection,
      bobPreCompressedTokenAccounts,
      fixedMint.publicKey,
      bn(70),
      bob.publicKey,
      charlie.publicKey,
    );

    const bobPreCompressedTokenAccounts2 =
      await getCompressedTokenAccountsFromMockRpc(
        connection,
        bob.publicKey,
        fixedMint.publicKey,
      );
    await transfer(
      connection,
      payer,
      fixedMint.publicKey,
      10,
      bob,
      charlie.publicKey,
      merkleTree,
    );

    await assertTransfer(
      connection,
      bobPreCompressedTokenAccounts2,
      fixedMint.publicKey,
      bn(10),
      bob.publicKey,
      charlie.publicKey,
    );

    const charliePreCompressedTokenAccounts3 =
      await getCompressedTokenAccountsFromMockRpc(
        connection,
        charlie.publicKey,
        fixedMint.publicKey,
      );
    await transfer(
      connection,
      payer,
      fixedMint.publicKey,
      5,
      charlie,
      bob.publicKey,
      merkleTree,
    );

    await assertTransfer(
      connection,
      charliePreCompressedTokenAccounts3,
      fixedMint.publicKey,
      bn(5),
      charlie.publicKey,
      bob.publicKey,
    );

    /// c->b #2
    const charliePreCompressedTokenAccounts2 =
      await getCompressedTokenAccountsFromMockRpc(
        connection,
        charlie.publicKey,
        fixedMint.publicKey,
      );
    console.log(
      JSON.stringify(
        charliePreCompressedTokenAccounts2.map((x) =>
          x.parsed.amount.toString(),
        ),
      ),
    );

    /// c->b #3 (merge 2 utxos)
    await transfer(
      connection,
      payer,
      fixedMint.publicKey,
      74, // all rem -1
      charlie,
      bob.publicKey,
      merkleTree,
    );

    await assertTransfer(
      connection,
      charliePreCompressedTokenAccounts2,
      fixedMint.publicKey,
      bn(74),
      charlie.publicKey,
      bob.publicKey,
    );

    await expect(
      transfer(
        connection,
        payer,
        fixedMint.publicKey,
        10000,
        bob,
        charlie.publicKey,
        merkleTree,
      ),
    ).rejects.toThrow('Not enough balance for transfer');
  });

  it.skip('should return validityProof from prover server', async () => {
    const rpc = await getMockRpc(connection);
    const compressedTokenAccounts = await getCompressedTokenAccountsFromMockRpc(
      connection,
      charlie.publicKey,
      fixedMint.publicKey,
    );
    const utxoHashes = compressedTokenAccounts.map(
      (utxo: UtxoWithParsedTokenTlvData) => utxo.merkleContext.hash,
    );
    await rpc.getValidityProof(utxoHashes);
  });
  /// TODO: move these as unit tests to program.ts
  it.skip('should create mint', async () => {
    const rentExemptBalance = SPL_TOKEN_MINT_RENT_EXEMPT_BALANCE;

    const ixs = await CompressedTokenProgram.createMint({
      feePayer: payer.publicKey,
      mint: fixedMint.publicKey,
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
    tx.sign([payer, fixedMint]);

    const txId = await sendAndConfirmTx(connection, tx);

    const poolAccount = CompressedTokenProgram.deriveTokenPoolPda(
      fixedMint.publicKey,
    );
    await assertMintCreated(
      fixedMint.publicKey,
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
      mint: fixedMint.publicKey,
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
      } tokens (mint: ${fixedMint.publicKey.toBase58()}) to bob \n txId: ${txId}`,
    );
    /// TODO: assert output utxos after implementing proper beet serde
  });
});
