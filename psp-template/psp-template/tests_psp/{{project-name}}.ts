import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  Utxo,
  TransactionParameters,
  Provider as LightProvider,
  confirmConfig,
  Action,
  TestRelayer,
  User,
  ProgramUtxoBalance,
  airdropSol,
  ProgramParameters
} from "@lightprotocol/zk.js";
import {
  SystemProgram,
  PublicKey,
  Keypair,
} from "@solana/web3.js";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { IDL } from "../target/types/{{rust-name}}";
const path = require("path");

const verifierProgramId = new PublicKey(
  "{{program-id}}",
);
var POSEIDON;

const RPC_URL = "http://127.0.0.1:8899";

describe("Test {{project-name}}", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);

  before(async () => {
    POSEIDON = await buildPoseidonOpt();
  });


  it("Create and Spend Program Utxo ", async () => {
    const wallet = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: wallet.publicKey,
    });

    let relayer = new TestRelayer({
      relayerPubkey: wallet.publicKey,
      relayerRecipientSol:  wallet.publicKey,
      relayerFee: new BN(100000),
      payer: wallet
    });

    // The light provider is a connection and wallet abstraction.
    // The wallet is used to derive the seed for your shielded keypair with a signature.
    var lightProvider = await LightProvider.init({ wallet, url: RPC_URL, relayer, confirmConfig });
    lightProvider.addVerifierProgramPublickeyToLookUpTable(TransactionParameters.getVerifierProgramId(IDL));

    const user: User = await User.init({ provider: lightProvider });

    const outputUtxoSol = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: user.account.pubkey,
      amounts: [new BN(1_000_000)],
      appData: { x: new BN(1), y: new BN(1) },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable: lightProvider.lookUpTables.verifierProgramLookupTable
    });

    const testInputsShield = {
      utxo: outputUtxoSol,
      action: Action.SHIELD,
    }

    let storeTransaction = await user.storeAppUtxo({
      appUtxo: testInputsShield.utxo,
      action: testInputsShield.action,
    });
    console.log("store program utxo transaction hash ", storeTransaction.txHash);

    const programUtxoBalance: Map<string, ProgramUtxoBalance> =
      await user.syncStorage(IDL);
    const shieldedUtxoCommitmentHash =
      testInputsShield.utxo.getCommitment(POSEIDON);
    const inputUtxo = programUtxoBalance
      .get(verifierProgramId.toBase58())
      .tokenBalances.get(testInputsShield.utxo.assets[1].toBase58())
      .utxos.get(shieldedUtxoCommitmentHash);

    Utxo.equal(POSEIDON, inputUtxo, testInputsShield.utxo, true);

    const circuitPath = path.join("build-circuit");

    const programParameters: ProgramParameters = {
      inputs: {
        x: inputUtxo.appData.x,
        y: inputUtxo.appData.y,
        publicZ: inputUtxo.appData.x.add(inputUtxo.appData.y)
      },
      verifierIdl: IDL,
      path: circuitPath,
      circuitName: "{{circom-name-camel-case}}",
    };

    let { txHash } = await user.executeAppUtxo({
      appUtxos: [inputUtxo],
      programParameters,
      action: Action.TRANSFER,
    });
    console.log("transaction hash ", txHash);
    const utxoSpent = await user.getUtxo(inputUtxo.getCommitment(POSEIDON), true, IDL);
    assert.equal(utxoSpent.status, "spent");
    Utxo.equal(POSEIDON, utxoSpent.utxo, inputUtxo, true);
  });
});
