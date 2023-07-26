import { assert, expect } from "chai";

import { Keypair as SolanaKeypair } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { it } from "mocha";

import {
  Relayer,
  RelayerError,
  RelayerErrorCode,
  TOKEN_ACCOUNT_FEE,
} from "../src";

process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
let mockKeypair = SolanaKeypair.generate();
let mockKeypair1 = SolanaKeypair.generate();
let relayerFee = new anchor.BN("123214");
let relayerRecipientSol = SolanaKeypair.generate().publicKey;

describe("Test Relayer Functional", () => {
  it("Relayer Deposit", () => {
    let relayer = new Relayer(mockKeypair.publicKey, mockKeypair1.publicKey);
    assert.equal(
      relayer.accounts.lookUpTable.toBase58(),
      mockKeypair1.publicKey.toBase58(),
    );
    assert.equal(
      relayer.accounts.relayerPubkey.toBase58(),
      mockKeypair.publicKey.toBase58(),
    );
    assert.equal(relayer.relayerFee.toString(), "0");
  });

  it("Relayer Transfer/Withdrawal", () => {
    let relayer = new Relayer(
      mockKeypair.publicKey,
      mockKeypair1.publicKey,
      relayerRecipientSol,
      relayerFee,
    );
    assert.equal(
      relayer.accounts.lookUpTable.toBase58(),
      mockKeypair1.publicKey.toBase58(),
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
    let relayer = new Relayer(mockKeypair.publicKey, mockKeypair1.publicKey);
    assert.equal(relayer.relayerFee.toString(), "0");
    assert.equal(
      TOKEN_ACCOUNT_FEE.toNumber(),
      relayer.getRelayerFee(true).toNumber(),
    );
    assert.equal(
      new anchor.BN(0).toNumber(),
      relayer.getRelayerFee(false).toNumber(),
    );
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
      new Relayer(
        mockKeypair.publicKey,
        mockKeypair1.publicKey,
        relayerRecipientSol,
      );
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
      new Relayer(
        mockKeypair.publicKey,
        mockKeypair1.publicKey,
        undefined,
        relayerFee,
      );
    })
      .to.throw(RelayerError)
      .includes({
        code: RelayerErrorCode.RELAYER_RECIPIENT_UNDEFINED,
        functionName: "constructor",
      });
  });
});
