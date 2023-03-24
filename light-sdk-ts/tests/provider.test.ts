import { assert, expect } from "chai";
let circomlibjs = require("circomlibjs");
import {
  SystemProgram,
  Keypair as SolanaKeypair,
  PublicKey,
  Connection,
} from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";
import { buildPoseidonOpt } from "circomlibjs";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
import { Account } from "../src/account";
import { Utxo } from "../src/utxo";
import {
  FEE_ASSET,
  hashAndTruncateToCircuit,
  Provider as LightProvider,
  MINT,
  Transaction,
  TransactionParameters,
  VerifierZero,
  TransactionErrorCode,
  Action,
  TransactioParametersError,
  TransactionParametersErrorCode,
  Relayer,
  FIELD_SIZE,
  merkleTreeProgramId,
  VerifierTwo,
  VerifierOne,
  AUTHORITY,
  TransactionError,
  ProviderErrorCode,
  SolMerkleTreeErrorCode,
  ADMIN_AUTH_KEYPAIR,
  MERKLE_TREE_KEY,
  DEFAULT_ZERO,
  ProviderError,
  useWallet,
} from "../src";
import { MerkleTree } from "../src/merkleTree/merkleTree";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const verifiers = [new VerifierZero(), new VerifierOne(), new VerifierTwo()];

describe("Test Provider Functional", () => {
  let seed32 = new Uint8Array(32).fill(1).toString();
  let depositAmount = 20_000;
  let depositFeeAmount = 10_000;

  let mockPubkey = SolanaKeypair.generate().publicKey;
  let mockPubkey1 = SolanaKeypair.generate().publicKey;
  let mockPubkey2 = SolanaKeypair.generate().publicKey;
  let mockPubkey3 = SolanaKeypair.generate().publicKey;
  let poseidon, lightProvider: LightProvider;

  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
  });

  it("Mock Provider", async () => {
    const lightProviderMock = await LightProvider.loadMock();
    assert.equal(lightProviderMock.wallet.node_wallet,true);
    assert.equal(
      lightProviderMock.wallet?.publicKey.toBase58(),
      ADMIN_AUTH_KEYPAIR.publicKey.toBase58(),
    );
    assert.equal(lightProviderMock.url, "mock");
    assert(lightProviderMock.poseidon);
    assert(lightProviderMock.lookUpTable);
    assert.equal(
      lightProviderMock.solMerkleTree?.pubkey.toBase58(),
      MERKLE_TREE_KEY.toBase58(),
    );
    assert.equal(lightProviderMock.solMerkleTree?.merkleTree.levels, 18);
    assert.equal(
      lightProviderMock.solMerkleTree?.merkleTree.zeroElement,
      DEFAULT_ZERO,
    );
  });

  it("KEYPAIR_UNDEFINED Provider", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      LightProvider.initialize(),
      ProviderErrorCode.KEYPAIR_UNDEFINED,
    );
  });

  it("CONNECTION_DEFINED", async () => {
    expect(() => {
      // @ts-ignore
      new LightProvider({ wallet: ADMIN_AUTH_KEYPAIR, connection: {} });
    })
      .to.throw(ProviderError)
      .includes({
        code: ProviderErrorCode.CONNECTION_DEFINED,
        functionName: "constructor",
      });
  });

  it("WALLET_UNDEFINED", async () => {
    expect(() => {
      // @ts-ignore
      new LightProvider({});
    })
      .to.throw(ProviderError)
      .includes({
        code: ProviderErrorCode.WALLET_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("CONNECTION_UNDEFINED", async () => {
    expect(() => {
      // @ts-ignore
      new LightProvider({ wallet: {} });
    })
      .to.throw(ProviderError)
      .includes({
        code: ProviderErrorCode.CONNECTION_UNDEFINED,
        functionName: "constructor",
      });
  });

  it("KEYPAIR_UNDEFINED browser", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      LightProvider.initialize(),
      ProviderErrorCode.KEYPAIR_UNDEFINED,
    );
  });

  it("KEYPAIR_UNDEFINED browser", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      LightProvider.initialize(),
      ProviderErrorCode.KEYPAIR_UNDEFINED,
    );
  });

  it("CONNECTION_UNDEFINED browser", async () => {
    const mockKeypair = SolanaKeypair.generate();

    const wallet = useWallet(mockKeypair)

    await chai.assert.isRejected(
      // @ts-ignore
      LightProvider.initialize(wallet),
      ProviderErrorCode.CONNECTION_UNDEFINED,
    );
  });
});
