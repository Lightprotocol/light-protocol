import * as anchor from "@coral-xyz/anchor";
import {
  Utxo,
  Provider as LightProvider,
  confirmConfig,
  Action,
  TestRelayer,
  User,
  airdropSol,
  ProgramParameters,
  FIELD_SIZE,
  Relayer,
  Account,
  sendVersionedTransactions,
  STANDARD_SHIELDED_PUBLIC_KEY,
  BN_0,
  BN_1,
} from "@lightprotocol/zk.js";
import {
  SystemProgram,
  PublicKey,
  Keypair,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { IDL, Swaps } from "../target/types/swaps";
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
const path = require("path");

const verifierProgramId = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
);

let POSEIDON: any, RELAYER: TestRelayer;
const RPC_URL = "http://127.0.0.1:8899";

type SwapParameters = {
  swapMakerCommitmentHash?: BN;
  slot: BN;
  swapTakerCommitmentHash: BN;
  amountFrom: BN;
  amountTo: BN;
  userPubkey: BN;
};

class Swap {
  swapParameters: SwapParameters;
  programUtxo: Utxo;
  pda: PublicKey;

  constructor(
    swapParameters: SwapParameters,
    programUtxo: Utxo,
    pda: PublicKey
  ) {
    this.swapParameters = swapParameters;
    this.programUtxo = programUtxo;
    this.pda = pda;
  }

  static generateCommitmentHash(
    provider: LightProvider,
    swapParameters: SwapParameters
  ) {
    return new BN(
      provider.poseidon.F.toString(
        provider.poseidon([
          swapParameters.slot,
          swapParameters.swapTakerCommitmentHash,
          swapParameters.amountFrom,
          swapParameters.amountTo,
        ])
      )
    );
  }

  static async create(
    amountFrom: BN,
    amountTo: BN,
    lightProvider: LightProvider
  ) {
    const slot = await lightProvider.connection.getSlot();
    const swapParameters: SwapParameters = {
      slot: new BN(slot),
      amountFrom,
      amountTo,
      swapTakerCommitmentHash: BN_0,
      userPubkey: BN_0,
    };
    swapParameters.swapMakerCommitmentHash = Swap.generateCommitmentHash(
      lightProvider,
      swapParameters
    );
    const programUtxo = new Utxo({
      poseidon: POSEIDON,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      amounts: [swapParameters.amountFrom],
      appData: {
        swapCommitmentHash: swapParameters.swapMakerCommitmentHash,
        userPubkey: swapParameters.userPubkey,
      },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    let seed = swapParameters.swapMakerCommitmentHash.toArray("le", 32);
    const pda = findProgramAddressSync(
      [Buffer.from(seed)],
      new PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS")
    )[0];

    return new Swap(swapParameters, programUtxo, pda);
  }

  static async join(
    swapCommitmentHash: BN,
    amountFrom: BN,
    amountTo: BN,
    lightProvider: LightProvider,
    account: Account
  ) {
    const slot = await lightProvider.connection.getSlot();
    const swapParameters: SwapParameters = {
      slot: new BN(slot),
      amountFrom,
      amountTo,
      swapTakerCommitmentHash: swapCommitmentHash,
      userPubkey: account.pubkey,
    };
    swapParameters.swapMakerCommitmentHash = Swap.generateCommitmentHash(
      lightProvider,
      swapParameters
    );

    const programUtxo = new Utxo({
      poseidon: lightProvider.poseidon,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      amounts: [swapParameters.amountTo],
      appData: {
        swapCommitmentHash: swapParameters.swapMakerCommitmentHash,
        userPubkey: account.pubkey,
      },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        lightProvider.lookUpTables.verifierProgramLookupTable,
    });

    let seed = swapCommitmentHash.toArray("le", 32);
    const pda = findProgramAddressSync(
      [Buffer.from(seed)],
      new PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS")
    )[0];
    return new Swap(swapParameters, programUtxo, pda);
  }
}

class Participant {
  user: User;
  swap?: Swap;
  pspInstance: anchor.Program<Swaps>;

  constructor(user: User) {
    this.user = user;
    this.pspInstance = new anchor.Program(
      IDL,
      new PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"),
      user.provider.provider
    );
  }

  static async init(
    provider: anchor.AnchorProvider,
    relayer: TestRelayer | Relayer
  ) {
    const wallet = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e11,
      recipientPublicKey: wallet.publicKey,
    });

    // The light provider is a connection and wallet abstraction.
    // The wallet is used to derive the seed for your shielded keypair with a signature.
    let lightProvider = await LightProvider.init({
      wallet,
      url: RPC_URL,
      relayer,
      confirmConfig,
    });
    // lightProvider.addVerifierProgramPublickeyToLookUpTable(TransactionParameters.getVerifierProgramId(IDL));
    return new Participant(await User.init({ provider: lightProvider }));
  }

  async closeOffer() {
    if (!this.swap) {
      throw new Error("Swap is already closed.");
    }
    let tx = await this.pspInstance.methods
      .closeSwap()
      .accounts({
        swapPda: this.swap.pda,
        signer: this.user.provider.wallet.publicKey,
      })
      .instruction();

    await sendVersionedTransactions(
      [tx],
      this.user.provider.connection,
      this.user.provider.lookUpTables.versionedTransactionLookupTable,
      this.user.provider.wallet
    );
  }
  async createSwap(
    amountFrom: BN,
    amountTo: BN,
    action: Action = Action.SHIELD
  ) {
    if (this.swap) {
      throw new Error("A swap is already in progress.");
    }
    this.swap = await Swap.create(amountFrom, amountTo, this.user.provider);

    const txHash = await this.user.storeAppUtxo({
      appUtxo: this.swap.programUtxo,
      action,
    });

    const borshCoder = new anchor.BorshAccountsCoder(IDL);
    const serializationObject = {
      ...this.swap.programUtxo,
      ...this.swap.programUtxo.appData,
      accountEncryptionPublicKey: this.swap.programUtxo.encryptionPublicKey,
      accountShieldedPublicKey: this.swap.programUtxo.publicKey,
    };
    const utxoBytes = (
      await borshCoder.encode("utxo", serializationObject)
    ).subarray(8);

    let tx = await this.pspInstance.methods
      .createSwap(utxoBytes)
      .accounts({
        swapPda: this.swap.pda,
        signer: this.user.provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .instruction();

    let txHash2 = await sendVersionedTransactions(
      [tx],
      this.user.provider.connection,
      this.user.provider.lookUpTables.versionedTransactionLookupTable,
      this.user.provider.wallet
    );

    return {
      swap: this.swap,
      txHashStoreAppUtxo: txHash,
      txHashCreateSwap: txHash2,
    };
  }

  async takeOffer(
    swapCommitmentHash: BN,
    amountFrom: BN,
    amountTo: BN,
    action: Action = Action.SHIELD
  ) {
    if (this.swap) {
      throw new Error("A swap is already in progress.");
    }
    this.swap = await Swap.join(
      swapCommitmentHash,
      amountFrom,
      amountTo,
      this.user.provider,
      this.user.account
    );
    const txHash = await this.user.storeAppUtxo({
      appUtxo: this.swap.programUtxo,
      action,
    });
    const swapPdaAccountInfo = await this.pspInstance.account.swapPda.fetch(
      this.swap.pda
    );
    // @ts-ignore anchor type is not represented correctly
    if (swapPdaAccountInfo.isJoinable === false) {
      throw new Error("Swap is not joinable");
    }

    const borshCoder = new anchor.BorshAccountsCoder(IDL);
    const serializationObject = {
      ...this.swap.programUtxo,
      ...this.swap.programUtxo.appData,
      accountEncryptionPublicKey: this.swap.programUtxo.encryptionPublicKey,
      accountShieldedPublicKey: this.swap.programUtxo.publicKey,
    };
    const utxoBytes = (
      await borshCoder.encode("utxo", serializationObject)
    ).subarray(8);

    const tx = await this.pspInstance.methods
      .joinSwap(utxoBytes, this.swap.swapParameters.slot)
      .accounts({
        swapPda: this.swap.pda,
        signer: this.user.provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    let txHash2 = await sendVersionedTransactions(
      [tx],
      this.user.provider.connection,
      this.user.provider.lookUpTables.versionedTransactionLookupTable,
      this.user.provider.wallet
    );

    return {
      swap: this.swap,
      txHashStoreAppUtxo: txHash,
      txHashCreateSwap: txHash2,
    };
  }

  async execute(programUtxo?: Utxo) {
    const swapPdaAccountInfo = await this.pspInstance.account.swapPda.fetch(
      this.swap.pda
    );
    // @ts-ignore anchor type is not represented correctly
    if (swapPdaAccountInfo.isJoinable === true) {
      throw new Error("Swap is joinable not executable");
    }
    // @ts-ignore anchor type is not represented correctly
    const swapTakeParameters = {
      // @ts-ignore anchor type is not represented correctly
      swapCommitmentHash:
        swapPdaAccountInfo.swap.swapTakerProgramUtxo.swapCommitmentHash,
      // @ts-ignore anchor type is not represented correctly
      slot: swapPdaAccountInfo.swap.slot,
      // @ts-ignore anchor type is not represented correctly
      userPubkey: swapPdaAccountInfo.swap.swapTakerProgramUtxo.userPubkey,
    };
    const takerProgramUtxo = new Utxo({
      poseidon: this.user.provider.poseidon,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      // @ts-ignore anchor type is not represented correctly
      amounts: [swapPdaAccountInfo.swap.swapTakerProgramUtxo.amounts[0]],
      appData: {
        swapCommitmentHash: swapTakeParameters.swapCommitmentHash,
        userPubkey: swapTakeParameters.userPubkey,
      },
      appDataIdl: IDL,
      verifierAddress: new PublicKey(
        "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
      ),
      assetLookupTable: this.user.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.user.provider.lookUpTables.verifierProgramLookupTable,
      blinding: swapPdaAccountInfo.swap.swapTakerProgramUtxo.blinding,
    });
    Utxo.equal(
      this.user.provider.poseidon,
      takerProgramUtxo,
      programUtxo,
      false
    );
    const circuitPath = path.join("build-circuit");
    // We use getBalance to sync the current merkle tree
    await this.user.getBalance();
    const merkleTree = this.user.provider.solMerkleTree.merkleTree;
    this.swap.programUtxo.index = merkleTree.indexOf(
      this.swap.programUtxo.getCommitment(this.user.provider.poseidon)
    );
    takerProgramUtxo.index = merkleTree.indexOf(
      takerProgramUtxo.getCommitment(this.user.provider.poseidon)
    );

    const programParameters: ProgramParameters = {
      inputs: {
        publicSwapCommitment0: this.swap.swapParameters.swapMakerCommitmentHash,
        publicSwapCommitment1: takerProgramUtxo.appData.swapCommitmentHash,
        swapCommitmentHash: [
          this.swap.swapParameters.swapMakerCommitmentHash,
          swapTakeParameters.swapCommitmentHash,
        ],
        slot: [this.swap.swapParameters.slot, swapTakeParameters.slot],
        amountFrom: this.swap.swapParameters.amountFrom,
        amountTo: this.swap.swapParameters.amountTo,
        userPubkey: [
          this.swap.swapParameters.userPubkey,
          takerProgramUtxo.appData.userPubkey,
        ],
        isTakerOutUtxo: [[BN_0, BN_1, BN_0, BN_0]],
      },
      verifierIdl: IDL,
      path: circuitPath,
      accounts: {
        swapPda: this.swap.pda,
      },
      circuitName: "swaps",
    };

    const makerOutUtxo = new Utxo({
      poseidon: this.user.provider.poseidon,
      assets: [SystemProgram.programId],
      publicKey: this.user.account.pubkey,
      amounts: [this.swap.swapParameters.amountTo],
      assetLookupTable: this.user.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.user.provider.lookUpTables.verifierProgramLookupTable,
    });
    const takerOutUtxo = new Utxo({
      poseidon: this.user.provider.poseidon,
      assets: [SystemProgram.programId],
      publicKey: swapTakeParameters.userPubkey,
      encryptionPublicKey: new Uint8Array(
        swapPdaAccountInfo.swap.swapTakerProgramUtxo.accountEncryptionPublicKey
      ),
      amounts: [this.swap.swapParameters.amountFrom],
      assetLookupTable: this.user.provider.lookUpTables.assetLookupTable,
      verifierProgramLookupTable:
        this.user.provider.lookUpTables.verifierProgramLookupTable,
      blinding: swapTakeParameters.userPubkey
        .add(swapTakeParameters.userPubkey)
        .mod(FIELD_SIZE),
    });

    let payerUtxo = this.user.getAllUtxos();

    console.log(
      "maker balance before swap execution: " +
        (await this.user.getBalance()).totalSolBalance.toNumber() /
          LAMPORTS_PER_SOL
    );

    let { txHash } = await this.user.executeAppUtxo({
      appUtxos: [this.swap.programUtxo, takerProgramUtxo],
      inUtxos: [payerUtxo[0]],
      outUtxos: [makerOutUtxo, takerOutUtxo],
      programParameters,
      action: Action.TRANSFER,
      addOutUtxos: true,
      shuffleEnabled: false,
    });

    return { txHash };
  }
}

describe("Test swaps", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);

  before(async () => {
    POSEIDON = await buildPoseidonOpt();
    const relayerWallet = Keypair.generate();
    await airdropSol({
      connection: provider.connection,
      lamports: 1e11,
      recipientPublicKey: relayerWallet.publicKey,
    });
    RELAYER = new TestRelayer({
      relayerPubkey: relayerWallet.publicKey,
      relayerRecipientSol: relayerWallet.publicKey,
      relayerFee: new BN(100000),
      payer: relayerWallet,
    });
  });

  it("Swap test", async () => {
    const swapMaker = await Participant.init(provider, RELAYER);
    await swapMaker.user.shield({
      publicAmountSol: 10,
      token: "SOL",
    });

    console.log(
      "maker balance: " +
        (await swapMaker.user.getBalance()).totalSolBalance.toNumber() /
          LAMPORTS_PER_SOL
    );

    let res = await swapMaker.createSwap(
      new BN(0.5 * LAMPORTS_PER_SOL),
      new BN(0.25 * LAMPORTS_PER_SOL)
    );
    console.log("Swap offer created");

    const swapTaker = await Participant.init(provider, RELAYER);
    await swapTaker.user.shield({
      publicAmountSol: 10,
      token: "SOL",
    });

    console.log(
      "taker balance: " +
        (await swapTaker.user.getBalance()).totalSolBalance.toNumber() /
          LAMPORTS_PER_SOL
    );

    await swapTaker.takeOffer(
      res.swap.swapParameters.swapMakerCommitmentHash,
      new BN(0.5 * LAMPORTS_PER_SOL),
      new BN(0.25 * LAMPORTS_PER_SOL)
    );

    console.log("Token shielded and offer taken");
    let offerRes = await swapMaker.execute(swapTaker.swap.programUtxo);
    console.log("Swap tx hash: ", offerRes.txHash);
    await swapMaker.closeOffer();

    console.log(
      "maker balance after swap: " +
        (await swapMaker.user.getBalance()).totalSolBalance.toNumber() /
          LAMPORTS_PER_SOL
    );

    console.log(
      "taker balance after swap: " +
        (await swapTaker.user.getBalance()).totalSolBalance.toNumber() /
          LAMPORTS_PER_SOL
    );
  });
});
