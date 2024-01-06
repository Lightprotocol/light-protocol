import { MultiSigClient } from "./client";
import { MultisigParams } from "./multisigParams";
import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import { assert } from "chai";
import { IDL } from "./types/multisig";
import {
  Account,
  Action,
  airdropSol,
  confirmConfig,
  Provider as LightProvider,
  TestRelayer,
  TransactionParameters,
  User,
  Utxo,
} from "@lightprotocol/zk.js";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Connection,
} from "@solana/web3.js";
import { buildEddsa, buildPoseidonOpt } from "circomlibjs";
import { WasmFactory } from "@lightprotocol/account.rs";

export class Multisig {
  client: MultiSigClient;

  constructor(client: MultiSigClient) {
    this.client = client;
  }

  static async createMultiSig(): Promise<Multisig> {
    const RPC_URL = "http://127.0.0.1:8899";

    process.env.ANCHOR_PROVIDER_URL = RPC_URL;
    process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

    const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
    anchor.setProvider(provider);
    const poseidon = await buildPoseidonOpt();
    const hasher = await WasmFactory.getInstance();
    let eddsa = await buildEddsa();

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
    let lightProvider = await LightProvider.init({
      wallet,
      url: RPC_URL,
      relayer,
      confirmConfig,
    });
    lightProvider.addVerifierProgramPublickeyToLookUpTable(
      TransactionParameters.getVerifierProgramId(IDL),
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
      lightProvider,
    );

    const multisig = new Multisig(client);
    return multisig;
  }
  async create() {
    console.log("multisig::create");
  }

  toString() {
    return this.client.multiSigParams.debugString();
  }

  async add() {
    console.log("multisig::add");
  }
  async approve() {
    console.log("multisig::approve");
  }
  async execute() {
    console.log("multisig::execute");
  }
}
