import { assert, expect } from "chai";

import { Keypair as SolanaKeypair, SystemProgram } from "@solana/web3.js";
import { AnchorProvider, BN } from "@coral-xyz/anchor";
import { it } from "mocha";

import {
  ADMIN_AUTH_KEYPAIR,
  Action,
  BN_0,
  BN_1,
  MerkleTreeConfig,
  ParsedIndexedTransaction,
  RPC_FEE,
  Rpc,
  RpcError,
  RpcErrorCode,
  SolMerkleTree,
  TOKEN_ACCOUNT_FEE,
  TestRpc,
  Utxo,
  VerifierConfig,
  confirmConfig,
  createOutUtxo,
  createRpcIndexedTransactionResponse,
  encryptOutUtxos,
  getIdsFromEncryptedUtxos,
  Account,
} from "../src";
import { WasmFactory } from "@lightprotocol/account.rs";
import { MerkleTree } from "@lightprotocol/circuit-lib.js";
import { bs58 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

const mockKeypair = SolanaKeypair.generate();
const mockKeypair1 = SolanaKeypair.generate();
const rpcFee = new BN("123214");
const rpcRecipientSol = SolanaKeypair.generate().publicKey;
const seed32 = bs58.encode(new Uint8Array(32).fill(1));

describe("Test Rpc Functional", () => {
  it("Rpc Shield", () => {
    const rpc = new Rpc(mockKeypair.publicKey, mockKeypair1.publicKey, BN_1);
    assert.equal(
      rpc.accounts.rpcRecipientSol.toBase58(),
      mockKeypair1.publicKey.toBase58(),
    );
    assert.equal(
      rpc.accounts.rpcPubkey.toBase58(),
      mockKeypair.publicKey.toBase58(),
    );
    assert.equal(rpc.rpcFee.toString(), "1");
  });

  it("Rpc Transfer/Unshield", () => {
    const rpc = new Rpc(mockKeypair.publicKey, rpcRecipientSol, rpcFee);
    assert.equal(
      rpc.accounts.rpcPubkey.toBase58(),
      mockKeypair.publicKey.toBase58(),
    );
    assert.equal(rpc.rpcFee.toString(), rpcFee.toString());
    assert.equal(
      rpc.accounts.rpcRecipientSol.toBase58(),
      rpcRecipientSol.toBase58(),
    );
  });

  it("Rpc ataCreationFee", () => {
    const rpc = new Rpc(mockKeypair.publicKey);
    assert.equal(rpc.rpcFee.toString(), "0");
    assert.equal(TOKEN_ACCOUNT_FEE.toNumber(), rpc.getRpcFee(true).toNumber());
    assert.equal(BN_0.toNumber(), rpc.getRpcFee(false).toNumber());
  });
});

describe("Test Rpc Errors", () => {
  it("RPC_PUBKEY_UNDEFINED", () => {
    expect(() => {
      // @ts-ignore
      new Rpc();
    })
      .to.throw(RpcError)
      .includes({
        code: RpcErrorCode.RPC_PUBKEY_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("RPC_FEE_UNDEFINED", () => {
    expect(() => {
      // @ts-ignore
      new Rpc(mockKeypair.publicKey, rpcRecipientSol);
    })
      .to.throw(RpcError)
      .includes({
        code: RpcErrorCode.RPC_FEE_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("RPC_RECIPIENT_UNDEFINED", () => {
    expect(() => {
      // @ts-ignore
      new Rpc(mockKeypair.publicKey, undefined, rpcFee);
    })
      .to.throw(RpcError)
      .includes({
        code: RpcErrorCode.RPC_RECIPIENT_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("getIds from encrypted utxos", async () => {
    const WASM = await WasmFactory.getInstance();

    const account = Account.random(WASM);
    const utxo = createOutUtxo({
      amounts: [new BN(1)],
      assets: [mockKeypair.publicKey],
      publicKey: account.keypair.publicKey,
      lightWasm: WASM,
    });
    const utxo2 = createOutUtxo({
      amounts: [new BN(2)],
      assets: [mockKeypair.publicKey],
      publicKey: account.keypair.publicKey,
      lightWasm: WASM,
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
      [SystemProgram.programId.toBase58()],
      WASM,
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
    const utxo = createOutUtxo({
      amounts: [new BN(1)],
      assets: [mockKeypair.publicKey],
      publicKey: account.keypair.publicKey,
      lightWasm: WASM,
    });
    const utxo2 = createOutUtxo({
      amounts: [new BN(2)],
      assets: [mockKeypair.publicKey],
      publicKey: account.keypair.publicKey,
      lightWasm: WASM,
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
      [SystemProgram.programId.toBase58()],
      WASM,
    );

    const merkleTree = new MerkleTree(18, WASM, [
      utxo.utxoHash,
      utxo2.utxoHash,
    ]);
    const solMerkleTree = new SolMerkleTree({
      pubkey: MerkleTreeConfig.getTransactionMerkleTreePda(),
      lightWasm: WASM,
      merkleTree,
    });

    const indexedTransaction: ParsedIndexedTransaction = {
      blockTime: 0,
      signer: mockKeypair.publicKey.toBase58(),
      signature: "",
      to: mockKeypair.publicKey.toBase58(),
      from: mockKeypair.publicKey.toBase58(),
      toSpl: mockKeypair.publicKey.toBase58(),
      fromSpl: mockKeypair.publicKey.toBase58(),
      verifier: mockKeypair.publicKey.toBase58(),
      rpcRecipientSol: mockKeypair.publicKey.toBase58(),
      type: Action.SHIELD,
      changeSolAmount: "0",
      publicAmountSol: "0",
      publicAmountSpl: "0",
      encryptedUtxos: Array.from(encryptedUtxos),
      leaves: [
        new BN(utxo.utxoHash).toArray("be", 32),
        new BN(utxo2.utxoHash).toArray("be", 32),
      ],
      firstLeafIndex: "0",
      nullifiers: [Array(32).fill(1), Array(32).fill(2)],
      rpcFee: "0",
      message: Array<number>(),
    };
    const rpcIndexedTransactionResponse = createRpcIndexedTransactionResponse(
      indexedTransaction,
      solMerkleTree,
    );

    assert.deepEqual(
      rpcIndexedTransactionResponse.transaction,
      indexedTransaction,
    );
    assert.deepEqual(
      rpcIndexedTransactionResponse.merkleProofs[0],
      solMerkleTree.merkleTree.path(0).pathElements,
    );
    assert.deepEqual(
      rpcIndexedTransactionResponse.merkleProofs[1],
      solMerkleTree.merkleTree.path(1).pathElements,
    );
    assert.equal(rpcIndexedTransactionResponse.leavesIndexes[0], 0);
    assert.equal(rpcIndexedTransactionResponse.leavesIndexes[1], 1);
  });

  it.skip("Index transaction", async () => {
    const WASM = await WasmFactory.getInstance();
    const rpcRecipientSol = SolanaKeypair.generate().publicKey;
    const provider = AnchorProvider.local(
      "http://127.0.0.1:8899",
      confirmConfig,
    );
    await provider.connection.requestAirdrop(rpcRecipientSol, 2e9);

    const RPC = new TestRpc({
      rpcPubkey: ADMIN_AUTH_KEYPAIR.publicKey,
      rpcRecipientSol,
      rpcFee: RPC_FEE,
      payer: ADMIN_AUTH_KEYPAIR,
      connection: provider.connection,
      lightWasm: WASM,
    });
    // prefix  4DyKUQ
    // prefix  3pvhXa
    const eventIdUtxo1 = await RPC.getEventById(
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

    const eventIdUtxo2 = await RPC.getEventById(
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
