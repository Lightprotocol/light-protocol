import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import {
  Account,
  Action,
  airdropSol,
  confirmConfig,
  Provider as LightProvider,
  TestRpc,
  TransactionParameters,
  User,
  Utxo,
} from "@lightprotocol/zk.js";
import { Keypair } from "@solana/web3.js";
import { IDL } from "../target/types/multisig";
import { buildEddsa, buildPoseidonOpt } from "circomlibjs";
import { MultiSigClient, printUtxo } from "../src";
import { WasmFactory } from "@lightprotocol/account.rs";

// let circomlibjs = require("circomlibjs");
// const path = require("path");
// const verifierProgramId = new PublicKey(
//   "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
// );

const RPC_URL = "http://127.0.0.1:8899";

describe("Test multisig", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);

  it.skip("Test Withdrawal Multisig", async () => {
    const hasher = await WasmFactory.getInstance();
    const poseidon = await buildPoseidonOpt();
    let eddsa = await buildEddsa();

    const wallet = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e10,
      recipientPublicKey: wallet.publicKey,
    });

    let rpc = new TestRpc({
      rpcPubkey: wallet.publicKey,
      rpcRecipientSol: wallet.publicKey,
      rpcFee: new BN(100000),
      payer: wallet,
    });

    // The light provider is a connection and wallet abstraction.
    // The wallet is used to derive the seed for your shielded keypair with a signature.
    let lightProvider = await LightProvider.init({
      wallet,
      url: RPC_URL,
      rpc,
      confirmConfig,
    });
    lightProvider.addVerifierProgramPublickeyToLookUpTable(
      TransactionParameters.getVerifierProgramId(IDL)
    );

    const user: User = await User.init({ provider: lightProvider });

    const keypair = new Account({
      hasher,
      seed: new Uint8Array(32).fill(1).toString(),
      eddsa,
    });

    const signers = [user.account, keypair];

    const client = await MultiSigClient.createMultiSigParameters(
      2,
      user.account,
      signers,
      hasher,
      poseidon,
      eddsa,
      lightProvider
    );

    console.log("------------------------------------------");
    console.log("\t Created Multisig ");
    console.log("------------------------------------------");
    console.log("The creator of the multisig creates a shared encryption key.");
    console.log(
      "The shared encryption key is encrypted to the encryption publickeys of all signers individually."
    );
    console.log(
      "The shared encryption key is used to encrypt all subsequent transactions."
    );
    console.log(
      "Together with the encrypted shared key,\n parameter data is encrypted to a shared encryption key and stored in a compressed account on Solana."
    );

    client.multiSigParams.print();
    console.log("------------------------------------------");
    console.log("\n\n");

    const withdrawalAmount = 1_000_000_0;
    let outputUtxo = client.createUtxo({ solAmount: new BN(withdrawalAmount) });

    // Deposit to multisig
    console.log("------------------------------------------");
    console.log("\t Depositing to Multisig ");
    console.log("------------------------------------------");
    console.log(
      "A normal light protocol deposit transaction creates a multisig utxo."
    );
    console.log("Every light transaction has input and output utxos.");
    console.log(
      "During transaction execution input utxos are invalidated, \n while output utxos are inserted into the merkle tree"
    );
    console.log("This is the multisig output utxo");
    console.log(printUtxo(outputUtxo, hasher, 0, "ouput"));

    await deposit(outputUtxo, user);
    console.log("DEPOSITED");
    console.log("------------------------------------------");
    console.log("\n\n");

    const inputUtxos = [outputUtxo];
    const outputUtxos = [];

    await client.createMultiSigTransaction({
      inputUtxos,
      outputUtxos,
      rpc,
      action: Action.UNSHIELD,
    });
    console.log("------------------------------------------");
    console.log("\t Created Multisig Transaction ");
    console.log("------------------------------------------");
    console.log(
      "The multisig transaction is encrypted to the shared encryption key and stored in a compressed account on Solana."
    );
    //    console.log(client.queuedTransactions[0]);
    const approvedTransaction = await client.approve(0);

    console.log("------------------------------------------");
    console.log("\tSigner 2 Client");
    console.log("------------------------------------------");
    console.log(
      " Signer 2 fetches the multisig configuration, transaction and the approval from Solana."
    );

    // creates a client object with the second signer
    const client1 = new MultiSigClient({
      provider: lightProvider,
      multiSigParams: client.multiSigParams,
      signer: keypair,
      poseidon,
      queuedTransactions: [approvedTransaction],
      eddsa,
      hasher,
    });
    // approves the multisig transaction
    await client1.approve(0);

    console.log("\n\n------------------------------------------");
    console.log("\t Executing Multisig Transaction ");
    console.log("------------------------------------------");

    await client1.execute(0);
    console.log("------------------------------------------\n");
  });

  async function deposit(utxo: Utxo, user: User) {
    let tx = await user.storeAppUtxo({
      appUtxo: utxo,
      action: Action.SHIELD,
    });
    console.log("store program utxo transaction hash ", tx.txHash);
  }
});
