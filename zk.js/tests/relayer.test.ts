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
import { WasmFactory } from "@lightprotocol/account.rs";
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
    const WASM = await WasmFactory.getInstance();

    const account = Account.random(WASM);
    const utxo = new Utxo({
      amounts: [new BN(1)],
      assets: [mockKeypair.publicKey],
      publicKey: account.keypair.publicKey,
      lightWasm: WASM,
      assetLookupTable: [SystemProgram.programId.toBase58()],
    });
    const utxo2 = new Utxo({
      amounts: [new BN(2)],
      assets: [mockKeypair.publicKey],
      publicKey: account.keypair.publicKey,
      lightWasm: WASM,
      assetLookupTable: [SystemProgram.programId.toBase58()],
    });
    const verifierConfig: VerifierConfig = {
      in: 2,
      out: 2,
    };
    const encryptedUtxos = await encryptOutUtxos(
      account,
      [utxo, utxo2],
      MerkleTreeConfig.getTransactionMerkleTreePda(),
      verifierConfig,
        WASM
    );
    const ids = getIdsFromEncryptedUtxos(Buffer.from(encryptedUtxos), 2);
    assert.equal(
      ids[0],
      bs58.encode(
        account.generateUtxoPrefixHash(
          MerkleTreeConfig.getTransactionMerkleTreePda(),
          0,
        ),
      ),
    );
    assert.equal(
      ids[1],
      bs58.encode(
        account.generateUtxoPrefixHash(
          MerkleTreeConfig.getTransactionMerkleTreePda(),
          1,
        ),
      ),
    );
  });

  it("create rpc index", async () => {
    const WASM = await WasmFactory.getInstance();

    const account = Account.random(WASM);
    const utxo = new Utxo({
      amounts: [new BN(1)],
      assets: [mockKeypair.publicKey],
      publicKey: account.keypair.publicKey,
      lightWasm: WASM,
      assetLookupTable: [SystemProgram.programId.toBase58()],
    });
    const utxo2 = new Utxo({
      amounts: [new BN(2)],
      assets: [mockKeypair.publicKey],
      publicKey: account.keypair.publicKey,
      lightWasm: WASM,
      assetLookupTable: [SystemProgram.programId.toBase58()],
    });
    const verifierConfig: VerifierConfig = {
      in: 2,
      out: 2,
    };
    const encryptedUtxos = await encryptOutUtxos(
      account,
      [utxo, utxo2],
      MerkleTreeConfig.getTransactionMerkleTreePda(),
      verifierConfig,
        WASM
    );

    const merkleTree = new MerkleTree(18, WASM, [
      utxo.getCommitment(WASM),
      utxo2.getCommitment(WASM),
    ]);
    const solMerkleTree = new SolMerkleTree({
      pubkey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      lightWasm: WASM,
      merkleTree,
    });

    const indexedTransaction: ParsedIndexedTransaction = {
      blockTime: 0,
      signer: mockKeypair.publicKey.toString(),
      signature: "",
      to: mockKeypair.publicKey.toString(),
      from: mockKeypair.publicKey.toString(),
      toSpl: mockKeypair.publicKey.toString(),
      fromSpl: mockKeypair.publicKey.toString(),
      verifier: mockKeypair.publicKey.toString(),
      relayerRecipientSol: mockKeypair.publicKey.toString(),
      type: Action.SHIELD,
      changeSolAmount: BN_0.toString(),
      publicAmountSol: BN_0.toString(),
      publicAmountSpl: BN_0.toString(),
      encryptedUtxos: Buffer.from(encryptedUtxos),
      leaves: [
        new BN(utxo.getCommitment(WASM)).toArray("be", 32),
        new BN(utxo2.getCommitment(WASM)).toArray("be", 32),
      ],
      firstLeafIndex: BN_0.toString(),
      nullifiers: [
        new BN(utxo.getNullifier({ lightWasm: WASM, account, index: 0 })),
        new BN(utxo2.getNullifier({ lightWasm: WASM, account, index: 1 })),
      ],
      relayerFee: BN_0.toString(),
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
    const WASM = await WasmFactory.getInstance();
    const seed = bs58.encode(new Uint8Array(32).fill(1));
    const ACCOUNT = Account.createFromSeed(WASM, seed);
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
      lightWasm: WASM,
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
