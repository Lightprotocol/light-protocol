import { assert, expect } from "chai";

import { Keypair as SolanaKeypair, SystemProgram } from "@solana/web3.js";
import { AnchorProvider, BN } from "@coral-xyz/anchor";
import { it } from "mocha";

import {
  ADMIN_AUTH_KEYPAIR,
  Account,
  Action,
  BN_0,
  BN_1,
  MerkleTreeConfig,
  ParsedIndexedTransaction,
  RELAYER_FEE,
  Relayer,
  RelayerError,
  RelayerErrorCode,
  SolMerkleTree,
  TOKEN_ACCOUNT_FEE,
  TestRelayer,
  Utxo,
  VerifierConfig,
  confirmConfig,
  createRpcIndexedTransaction,
  encryptOutUtxos,
  getIdsFromEncryptedUtxos,
} from "../src";
import { WasmHasher } from "@lightprotocol/account.rs";
import { MerkleTree, encrypt } from "@lightprotocol/circuit-lib.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const mockKeypair = SolanaKeypair.generate();
const mockKeypair1 = SolanaKeypair.generate();
const relayerFee = new BN("123214");
const relayerRecipientSol = SolanaKeypair.generate().publicKey;

describe("Test Relayer Functional", () => {
  it("Relayer Shield", () => {
    const relayer = new Relayer(
      mockKeypair.publicKey,
      mockKeypair1.publicKey,
      BN_1,
    );
    assert.equal(
      relayer.accounts.relayerRecipientSol.toBase58(),
      mockKeypair1.publicKey.toBase58(),
    );
    assert.equal(
      relayer.accounts.relayerPubkey.toBase58(),
      mockKeypair.publicKey.toBase58(),
    );
    assert.equal(relayer.relayerFee.toString(), "1");
  });

  it("Relayer Transfer/Unshield", () => {
    const relayer = new Relayer(
      mockKeypair.publicKey,
      relayerRecipientSol,
      relayerFee,
    );
    assert.equal(
      relayer.accounts.relayerPubkey.toBase58(),
      mockKeypair.publicKey.toBase58(),
    );
    assert.equal(relayer.relayerFee.toString(), relayerFee.toString());
    assert.equal(
      relayer.accounts.relayerRecipientSol.toBase58(),
      relayerRecipientSol.toBase58(),
    );
  });

  it("Relayer ataCreationFee", () => {
    const relayer = new Relayer(mockKeypair.publicKey);
    assert.equal(relayer.relayerFee.toString(), "0");
    assert.equal(
      TOKEN_ACCOUNT_FEE.toNumber(),
      relayer.getRelayerFee(true).toNumber(),
    );
    assert.equal(BN_0.toNumber(), relayer.getRelayerFee(false).toNumber());
  });
});

describe("Test Relayer Errors", () => {
  it("RELAYER_PUBKEY_UNDEFINED", () => {
    expect(() => {
      // @ts-ignore
      new Relayer();
    })
      .to.throw(RelayerError)
      .includes({
        code: RelayerErrorCode.RELAYER_PUBKEY_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("RELAYER_FEE_UNDEFINED", () => {
    expect(() => {
      // @ts-ignore
      new Relayer(mockKeypair.publicKey, relayerRecipientSol);
    })
      .to.throw(RelayerError)
      .includes({
        code: RelayerErrorCode.RELAYER_FEE_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("RELAYER_RECIPIENT_UNDEFINED", () => {
    expect(() => {
      // @ts-ignore
      new Relayer(mockKeypair.publicKey, undefined, relayerFee);
    })
      .to.throw(RelayerError)
      .includes({
        code: RelayerErrorCode.RELAYER_RECIPIENT_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("getIds from encrypted utxos", async () => {
    const HASHER = await WasmHasher.getInstance();

    const account = new Account({ hasher: HASHER });
    const utxo = new Utxo({
      amounts: [new BN(1)],
      assets: [mockKeypair.publicKey],
      publicKey: account.pubkey,
      hasher: HASHER,
      assetLookupTable: [SystemProgram.programId.toBase58()],
    });
    const utxo2 = new Utxo({
      amounts: [new BN(2)],
      assets: [mockKeypair.publicKey],
      publicKey: account.pubkey,
      hasher: HASHER,
      assetLookupTable: [SystemProgram.programId.toBase58()],
    });
    const verifierConfig: VerifierConfig = {
      in: 2,
      out: 2,
    };
    const encryptedUtxos = await encryptOutUtxos(
      HASHER,
      account,
      [utxo, utxo2],
      MerkleTreeConfig.getTransactionMerkleTreePda(),
      verifierConfig,
    );
    const ids = getIdsFromEncryptedUtxos(Buffer.from(encryptedUtxos), 2);
    assert.equal(
      ids[0],
      bs58.encode(
        account.generateUtxoPrefixHash(
          MerkleTreeConfig.getTransactionMerkleTreePda(),
          new BN(0),
          4,
          HASHER,
        ),
      ),
    );
    assert.equal(
      ids[1],
      bs58.encode(
        account.generateUtxoPrefixHash(
          MerkleTreeConfig.getTransactionMerkleTreePda(),
          new BN(1),
          4,
          HASHER,
        ),
      ),
    );
  });

  it("create rpc index", async () => {
    const HASHER = await WasmHasher.getInstance();

    const account = new Account({ hasher: HASHER });
    const utxo = new Utxo({
      amounts: [new BN(1)],
      assets: [mockKeypair.publicKey],
      publicKey: account.pubkey,
      hasher: HASHER,
      assetLookupTable: [SystemProgram.programId.toBase58()],
    });
    const utxo2 = new Utxo({
      amounts: [new BN(2)],
      assets: [mockKeypair.publicKey],
      publicKey: account.pubkey,
      hasher: HASHER,
      assetLookupTable: [SystemProgram.programId.toBase58()],
    });
    const verifierConfig: VerifierConfig = {
      in: 2,
      out: 2,
    };
    const encryptedUtxos = await encryptOutUtxos(
      HASHER,
      account,
      [utxo, utxo2],
      MerkleTreeConfig.getTransactionMerkleTreePda(),
      verifierConfig,
    );

    const merkleTree = new MerkleTree(18, HASHER, [
      utxo.getCommitment(HASHER),
      utxo2.getCommitment(HASHER),
    ]);
    const solMerkleTree = new SolMerkleTree({
      pubkey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      hasher: HASHER,
      merkleTree,
    });

    const indexedTransaction: ParsedIndexedTransaction = {
      blockTime: 0,
      signer: mockKeypair.publicKey,
      signature: "",
      to: mockKeypair.publicKey,
      from: mockKeypair.publicKey,
      toSpl: mockKeypair.publicKey,
      fromSpl: mockKeypair.publicKey,
      verifier: mockKeypair.publicKey,
      relayerRecipientSol: mockKeypair.publicKey,
      type: Action.SHIELD,
      changeSolAmount: BN_0,
      publicAmountSol: BN_0,
      publicAmountSpl: BN_0,
      encryptedUtxos: Buffer.from(encryptedUtxos),
      leaves: [
        new BN(utxo.getCommitment(HASHER)).toArray("be", 32),
        new BN(utxo2.getCommitment(HASHER)).toArray("be", 32),
      ],
      firstLeafIndex: BN_0,
      nullifiers: [
        new BN(utxo.getNullifier({ hasher: HASHER, account, index: 0 })),
        new BN(utxo2.getNullifier({ hasher: HASHER, account, index: 1 })),
      ],
      relayerFee: BN_0,
      message: Buffer.from(""),
    };
    const rpcIndexedTransaction = createRpcIndexedTransaction(
      indexedTransaction,
      solMerkleTree,
    );

    assert.deepEqual(rpcIndexedTransaction.transaction, indexedTransaction);
    assert.deepEqual(
      rpcIndexedTransaction.merkleProofs[0],
      solMerkleTree.merkleTree.path(0).pathElements,
    );
    assert.deepEqual(
      rpcIndexedTransaction.merkleProofs[1],
      solMerkleTree.merkleTree.path(1).pathElements,
    );
    assert.equal(rpcIndexedTransaction.leavesIndexes[0], 0);
    assert.equal(rpcIndexedTransaction.leavesIndexes[1], 1);
  });

  it.skip("Index transaction", async () => {
    const HASHER = await WasmHasher.getInstance();
    const seed = bs58.encode(new Uint8Array(32).fill(1));
    const ACCOUNT = new Account({
      hasher: HASHER,
      seed,
    });

    const relayerRecipientSol = SolanaKeypair.generate().publicKey;
    const provider = AnchorProvider.local(
      "http://127.0.0.1:8899",
      confirmConfig,
    );
    await provider.connection.requestAirdrop(relayerRecipientSol, 2e9);

    const RELAYER = new TestRelayer({
      relayerPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      relayerRecipientSol,
      relayerFee: RELAYER_FEE,
      payer: ADMIN_AUTH_KEYPAIR,
      connection: provider.connection,
      hasher: HASHER,
    });
    // prefix  4DyKUQ
    // prefix  3pvhXa
    const eventIdUtxo1 = await RELAYER.getEventById(
      MerkleTreeConfig.getTransactionMerkleTreePda(),
      "4DyKUQ",
      1,
    );
    if (!eventIdUtxo1) throw new Error("Event undefined");
    assert.equal(eventIdUtxo1.transaction.type, Action.SHIELD);
    assert.equal(
      bs58.encode(eventIdUtxo1.transaction.encryptedUtxos.slice(0, 4)),
      "4DyKUQ",
    );

    const eventIdUtxo2 = await RELAYER.getEventById(
      MerkleTreeConfig.getTransactionMerkleTreePda(),
      "3pvhXa",
      1,
    );
    if (!eventIdUtxo2) throw new Error("Event undefined");
    assert.equal(eventIdUtxo2.transaction.type, Action.SHIELD);
    assert.equal(
      bs58.encode(eventIdUtxo2.transaction.encryptedUtxos.slice(124, 128)),
      "3pvhXa",
    );
  });
});
