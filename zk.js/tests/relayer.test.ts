import { assert, expect } from "chai";

import { Keypair as SolanaKeypair } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";

import {
  BN_0,
  BN_1,
  Relayer,
  RelayerError,
  RelayerErrorCode,
  TOKEN_ACCOUNT_FEE,
} from "../src";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
const mockKeypair = SolanaKeypair.generate();
const mockKeypair1 = SolanaKeypair.generate();
const relayerFee = new BN("123214");
const relayerRecipientSol = SolanaKeypair.generate().publicKey;

describe("Test Relayer Functional", () => {
  it("Relayer Shield", () => {
    let relayer = new Relayer(
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
    let relayer = new Relayer(
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
});
