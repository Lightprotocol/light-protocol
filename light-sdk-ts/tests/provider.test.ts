import { assert, expect } from "chai";
let circomlibjs = require("circomlibjs");
import {
  SystemProgram,
  Keypair as SolanaKeypair,
  PublicKey,
  Connection,
} from "@solana/web3.js";
import { it } from "mocha";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
import {
  Provider as LightProvider,
  ProviderErrorCode,
  ADMIN_AUTH_KEYPAIR,
  TRANSACTION_MERKLE_TREE_KEY,
  DEFAULT_ZERO,
  ProviderError,
  useWallet,
} from "../src";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

describe("Test Provider Functional", () => {
  let poseidon

  before(async () => {
    poseidon = await circomlibjs.buildPoseidonOpt();
  });

  it("Mock Provider", async () => {
    const lightProviderMock = await LightProvider.loadMock();
    assert.equal(lightProviderMock.wallet.isNodeWallet, true);
    assert.equal(
      lightProviderMock.wallet?.publicKey.toBase58(),
      ADMIN_AUTH_KEYPAIR.publicKey.toBase58(),
    );
    assert.equal(lightProviderMock.url, "mock");
    assert(lightProviderMock.poseidon);
    assert(lightProviderMock.lookUpTable);
    assert.equal(
      lightProviderMock.solMerkleTree?.pubkey.toBase58(),
      TRANSACTION_MERKLE_TREE_KEY.toBase58(),
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
      LightProvider.init({}),
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

  it("CONNECTION_UNDEFINED browser", async () => {
    const mockKeypair = SolanaKeypair.generate();

    const wallet = useWallet(mockKeypair);

    await chai.assert.isRejected(
      // @ts-ignore
      LightProvider.init({ wallet }),
      ProviderErrorCode.CONNECTION_UNDEFINED,
    );
  });
});
