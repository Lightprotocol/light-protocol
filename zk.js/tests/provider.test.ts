import { assert, expect } from "chai";
import { SystemProgram, Keypair as SolanaKeypair } from "@solana/web3.js";
import { it } from "mocha";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
import {
  Provider as LightProvider,
  ProviderErrorCode,
  ADMIN_AUTH_KEYPAIR,
  DEFAULT_ZERO,
  ProviderError,
  MINT,
  MerkleTreeConfig,
} from "../src";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
process.env.LIGHT_PROTOCOL_ATOMIC_TRANSACTIONS = "true";

describe("Test Provider Functional", () => {
  it("Mock Provider", async () => {
    const lightProviderMock = await LightProvider.loadMock();
    assert.equal(lightProviderMock.wallet.isNodeWallet, true);
    assert.equal(
      lightProviderMock.wallet?.publicKey.toBase58(),
      ADMIN_AUTH_KEYPAIR.publicKey.toBase58(),
    );
    assert.equal(lightProviderMock.url, "mock");
    assert(lightProviderMock.poseidon);
    assert(lightProviderMock.lookUpTables.versionedTransactionLookupTable);
    assert.equal(
      lightProviderMock.solMerkleTree?.pubkey.toBase58(),
      MerkleTreeConfig.getTransactionMerkleTreePda().toBase58(),
    );
    assert.equal(lightProviderMock.solMerkleTree?.merkleTree.levels, 18);
    assert.equal(
      lightProviderMock.solMerkleTree?.merkleTree.zeroElement,
      DEFAULT_ZERO,
    );
    const additionalMint = SolanaKeypair.generate().publicKey;
    assert.equal(
      lightProviderMock.lookUpTables.assetLookupTable[0],
      SystemProgram.programId.toBase58(),
    );
    assert.equal(
      lightProviderMock.lookUpTables.assetLookupTable[1],
      MINT.toBase58(),
    );
    assert.equal(
      lightProviderMock.lookUpTables.verifierProgramLookupTable[0],
      SystemProgram.programId.toBase58(),
    );
    assert.equal(
      lightProviderMock.lookUpTables.verifierProgramLookupTable[1],
      "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
    );
    lightProviderMock.addAssetPublickeyToLookUpTable(additionalMint);
    assert.equal(
      lightProviderMock.lookUpTables.assetLookupTable[2],
      additionalMint.toBase58(),
    );
    lightProviderMock.addVerifierProgramPublickeyToLookUpTable(additionalMint);
    assert.equal(
      lightProviderMock.lookUpTables.verifierProgramLookupTable[2],
      additionalMint.toBase58(),
    );
  });

  it("KEYPAIR_UNDEFINED Provider", async () => {
    await chai.assert.isRejected(
      // @ts-ignore
      LightProvider.init({}),
      ProviderErrorCode.KEYPAIR_UNDEFINED,
    );
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
});
