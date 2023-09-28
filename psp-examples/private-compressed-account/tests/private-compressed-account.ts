import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  TransactionParameters,
  Provider as LightProvider,
  confirmConfig,
  TestRelayer,
  User,
  airdropSol,
} from "@lightprotocol/zk.js";
import { Keypair } from "@solana/web3.js";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { IDL } from "../target/types/private_compressed_account";
import { PoseidonCompressedAccount } from "../sdk";
var POSEIDON: any;
const RPC_URL = "http://127.0.0.1:8899";
var log = console.log;

describe("Test private-compressed-account", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);

  before(async () => {
    POSEIDON = await buildPoseidonOpt();
  });

  it.skip("Merkle Tree Update Circuits, 100 rounds", async () => {
    const compressedAccount = new PoseidonCompressedAccount(POSEIDON, IDL, 0);
    let insertValue = "12";
    let leafHash = POSEIDON.F.toString(POSEIDON([insertValue]));
    await compressedAccount.generateUpdateProof({ leafHash });
    for (let i = 0; i < 100; i++) {
      log(`i ${i}`);
      let insertValue1 = (i + 1).toString();
      let leafHash = POSEIDON.F.toString(POSEIDON([insertValue1]));
      console.time("fullProveAndParse");
      await compressedAccount.generateUpdateProof({ leafHash });
      console.timeEnd("fullProveAndParse");
    }
  });

  let compressedAccount: PoseidonCompressedAccount;
  let insertValue = "12";

  it("Inclusion Gt Circuit should succeed", async () => {
    compressedAccount = new PoseidonCompressedAccount(POSEIDON, IDL, 0);
    let leafHash = POSEIDON.F.toString(POSEIDON([insertValue]));
    await compressedAccount.generateUpdateProof({ leafHash });

    log("insertValue 12, refValue 12");
    await compressedAccount.generateInclusionProof({
      leafInput: insertValue,
      referenceValue: new BN("12"),
    });
    log("insertValue 12, refValue 11");
    await compressedAccount.generateInclusionProof({
      leafInput: insertValue,
      referenceValue: new BN("11"),
    });
  });
  it("Inclusion Gt Circuit should fail with Lt value", async () => {
    let throwed = false;
    try {
      log("insertValue 12, refValue 13");
      await compressedAccount.generateInclusionProof({
        leafInput: insertValue,
        referenceValue: new BN("13"),
      });
    } catch (error) {
      console.error("expected error ", error);
      throwed = true;
    }
    assert(throwed, "Should throw error");
  });

  it("Create and Spend Program Utxo loop", async () => {
    const wallet = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: wallet.publicKey,
    });

    let relayer = new TestRelayer({
      relayerPubkey: wallet.publicKey,
      relayerRecipientSol: wallet.publicKey,
      relayerFee: new BN(100000),
      payer: wallet,
    });

    // The light provider is a connection and wallet abstraction.
    // The wallet is used to derive the seed for your shielded keypair with a signature.
    var lightProvider = await LightProvider.init({
      wallet,
      url: RPC_URL,
      relayer,
      confirmConfig,
    });
    lightProvider.addVerifierProgramPublickeyToLookUpTable(
      TransactionParameters.getVerifierProgramId(IDL)
    );

    const user: User = await User.init({ provider: lightProvider });
    // User needs a shielded sol balance to pay for the transaction fees.
    await user.shield({ token: "SOL", publicAmountSol: "1" });

    const compressedAccount = new PoseidonCompressedAccount(
      POSEIDON,
      IDL,
      0,
      user
    );

    try {
      await compressedAccount.initMerkleTreeAccount();
    } catch (error) {
      console.error("error ", error);
      throw error;
    }
    log("merkle tree account initialized");

    let insertValue = "12";
    let { txHash } = await compressedAccount.insertLeaf(insertValue);
    log(`tx signatures: ${txHash.signatures}`);

    for (let i = 0; i < 10; i++) {
      let insertValue1 = (i + 1).toString();
      let { txHash } = await compressedAccount.insertLeaf(insertValue1);
      log(`i: ${i}, tx signatures: ${txHash.signatures}`);
      await compressedAccount.verifyInclusionGte({
        leafInput: insertValue,
        referenceValue: new BN("0"),
      });
    }
  });
});
