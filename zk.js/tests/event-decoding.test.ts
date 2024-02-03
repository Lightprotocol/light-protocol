import { assert } from "chai";
import { BN } from "@coral-xyz/anchor";
import { it } from "mocha";
import {
  Account,
  PublicTransactionIndexerEventAnchor,
  ParsingUtxoBeet,
  PublicTransactionIndexerEventBeet,
  PublicTestRpc,
  OutUtxo,
  createOutUtxo,
  MERKLE_TREE_SET,
} from "../src";
import { WasmFactory, LightWasm } from "@lightprotocol/account.rs";

import { Connection, PublicKey } from "@solana/web3.js";
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");

// Load chai-as-promised support
chai.use(chaiAsPromised);
process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
process.env.ANCHOR_PROVIDER_URL = "http://127.0.0.1:8899";

describe("Test Account Functional", () => {
  it.skip("Event decoding beet with nesting", () => {
    let eventData = Buffer.from([
      0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 109, 221, 129, 73, 139,
      106, 245, 1, 95, 37, 253, 254, 42, 73, 80, 210, 168, 244, 216, 161, 214,
      255, 186, 5, 127, 27, 153, 150, 88, 83, 57, 216, 9, 203, 246, 71, 191,
      105, 220, 115, 150, 213, 89, 159, 251, 168, 170, 12, 109, 193, 195, 87,
      133, 235, 169, 26, 45, 163, 109, 188, 150, 25, 27, 4, 186, 75, 28, 134,
      130, 66, 71, 1, 120, 191, 53, 6, 112, 226, 144, 72, 153, 77, 18, 183, 81,
      120, 126, 176, 3, 31, 224, 135, 206, 23, 242, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 101, 0, 0, 0, 0, 0, 0, 0, 109, 221,
      129, 73, 139, 106, 245, 1, 95, 37, 253, 254, 42, 73, 80, 210, 168, 244,
      216, 161, 214, 255, 186, 5, 127, 27, 153, 150, 88, 83, 57, 216, 9, 203,
      246, 71, 191, 105, 220, 115, 150, 213, 89, 159, 251, 168, 170, 12, 109,
      193, 195, 87, 133, 235, 169, 26, 45, 163, 109, 188, 150, 25, 27, 4, 233,
      32, 229, 180, 164, 130, 91, 46, 175, 162, 124, 62, 12, 178, 218, 65, 48,
      145, 33, 61, 213, 46, 245, 234, 117, 101, 226, 224, 164, 129, 199, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ]);
    let utxoData = Buffer.from([
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2,
      0, 0, 0, 0, 0, 0, 0, 14, 129, 15, 86, 229, 176, 155, 3, 8, 217, 125, 97,
      221, 115, 252, 160, 127, 236, 37, 229, 116, 84, 111, 6, 5, 182, 141, 86,
      7, 23, 246, 215, 32, 29, 80, 210, 12, 55, 172, 224, 206, 72, 234, 251, 4,
      214, 215, 140, 183, 183, 99, 27, 207, 3, 220, 89, 216, 44, 41, 209, 140,
      56, 131, 67, 39, 198, 68, 189, 235, 239, 241, 102, 35, 143, 232, 212, 114,
      70, 4, 218, 99, 143, 68, 214, 203, 128, 13, 222, 105, 96, 178, 31, 69,
      207, 119, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0,
    ]);

    let utxoTestData = new ParsingUtxoBeet(
      1,
      2,
      [3, 4],
      PublicKey.default,
      new Uint8Array(32),
      new Uint8Array(31),
      new Uint8Array(32),
      new Uint8Array(32),
      new Uint8Array(32),
      null,
    );
    let utxoTest = ParsingUtxoBeet.struct.serialize(utxoTestData);
    let utxoTest2 = ParsingUtxoBeet.struct.deserialize(utxoTest[0]);
    let res = ParsingUtxoBeet.struct.deserialize(utxoData);

    let event = new PublicTransactionIndexerEventBeet(
      [new Uint8Array(32)],
      [utxoTestData, utxoTestData],
      [1],
      new Uint8Array(32),
      new Uint8Array(32),
      1,
      new Array(32),
      new Uint8Array(32),
      PublicKey.default,
    );
    console.log("event ", event);
    let eventTest = PublicTransactionIndexerEventBeet.struct.serialize(event);
    console.log("eventTest ", eventTest);
    let eventTest2 = PublicTransactionIndexerEventBeet.struct.deserialize(
      eventTest[0],
    );
    console.log("eventTest2 ", eventTest2);
    // assert.deepEqual(event, eventTest2[0])
    console.log("const used = process.memoryUsage(); ", process.memoryUsage());
    console.log(
      "eventData ",
      PublicTransactionIndexerEventBeet.struct.deserialize(eventData),
    );
    console.log("const used = process.memoryUsage(); ", process.memoryUsage());
  });

  it.skip("Event decoding manual nesting", () => {
    let eventData = Buffer.from([
      0, 0, 0, 0, 2, 0, 0, 0, 224, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 25, 221,
      239, 163, 33, 36, 39, 48, 68, 103, 121, 132, 128, 194, 25, 137, 132, 118,
      18, 101, 209, 223, 247, 132, 75, 69, 83, 164, 76, 132, 107, 56, 9, 203,
      246, 71, 191, 105, 220, 115, 150, 213, 89, 159, 251, 168, 170, 12, 109,
      193, 195, 87, 133, 235, 169, 26, 45, 163, 109, 188, 150, 25, 27, 4, 186,
      75, 28, 134, 130, 66, 71, 1, 120, 191, 53, 6, 112, 226, 144, 72, 153, 77,
      18, 183, 81, 120, 126, 176, 3, 31, 224, 135, 206, 23, 242, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 224, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 101, 0, 0,
      0, 0, 0, 0, 0, 25, 221, 239, 163, 33, 36, 39, 48, 68, 103, 121, 132, 128,
      194, 25, 137, 132, 118, 18, 101, 209, 223, 247, 132, 75, 69, 83, 164, 76,
      132, 107, 56, 9, 203, 246, 71, 191, 105, 220, 115, 150, 213, 89, 159, 251,
      168, 170, 12, 109, 193, 195, 87, 133, 235, 169, 26, 45, 163, 109, 188,
      150, 25, 27, 4, 233, 32, 229, 180, 164, 130, 91, 46, 175, 162, 124, 62,
      12, 178, 218, 65, 48, 145, 33, 61, 213, 46, 245, 234, 117, 101, 226, 224,
      164, 129, 199, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0,
    ]);
    let utxoData = Buffer.from([
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2,
      0, 0, 0, 0, 0, 0, 0, 14, 129, 15, 86, 229, 176, 155, 3, 8, 217, 125, 97,
      221, 115, 252, 160, 127, 236, 37, 229, 116, 84, 111, 6, 5, 182, 141, 86,
      7, 23, 246, 215, 32, 29, 80, 210, 12, 55, 172, 224, 206, 72, 234, 251, 4,
      214, 215, 140, 183, 183, 99, 27, 207, 3, 220, 89, 216, 44, 41, 209, 140,
      56, 131, 67, 39, 198, 68, 189, 235, 239, 241, 102, 35, 143, 232, 212, 114,
      70, 4, 218, 99, 143, 68, 214, 203, 128, 13, 222, 105, 96, 178, 31, 69,
      207, 119, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
      0,
    ]);
    let decodedUtxo = new PublicTransactionIndexerEventAnchor().deserializeUtxo(
      utxoData,
    );
    console.log("decodedUtxo ", decodedUtxo);
    let decodedEvent = new PublicTransactionIndexerEventAnchor().deserialize(
      eventData,
    );
    console.log("decodedEvent ", decodedEvent);
  });

  // it.skip("Test rpc (needs running test validator", async () => {
  //   const connection = new Connection("http://localhost:8899");
  //   const lightWasm = await WasmFactory.getInstance();
  //   let rpc = new PublicTestRpc({ connection, lightWasm, merkleTreePublicKey: });
  //   const indexedTransactions = await rpc.getIndexedTransactions(connection);
  //   console.log("indexedTransactions ", indexedTransactions[0].outUtxos[0]);
  //   const owner = indexedTransactions[0].outUtxos[0].owner;
  //   console.log("owner ", owner);
  //   const utxos = await rpc.getAssetsByOwner(new BN(owner).toString());
  //   console.log("utxos ", utxos);
  // });
});
