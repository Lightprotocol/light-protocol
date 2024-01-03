import * as anchor from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  Account,
  Action,
  airdropSol,
  BN_0,
  BN_1,
  confirmConfig,
  ConfirmOptions,
  FIELD_SIZE,
  ProgramParameters,
  Provider as LightProvider,
  Relayer,
  sendVersionedTransactions,
  STANDARD_SHIELDED_PUBLIC_KEY,
  TestRelayer,
  User,
  Utxo,
} from "@lightprotocol/zk.js";
import { Hasher, WasmHasher } from "@lightprotocol/account.rs";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { IDL, RockPaperScissors } from "../target/types/rock_paper_scissors";
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";

const path = require("path");

const verifierProgramId = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
);

let HASHER: Hasher, RELAYER: TestRelayer;
const RPC_URL = "http://127.0.0.1:8899";
const GAME_AMOUNT = new BN(1e9);

enum Choice {
  ROCK = 0,
  PAPER = 1,
  SCISSORS = 2,
}

enum Winner {
  PLAYER1 = "PLAYER1",
  PLAYER2 = "PLAYER2",
  DRAW = "DRAW",
}

type GameParameters = {
  gameCommitmentHash?: BN;
  choice: Choice;
  slot: BN;
  player2CommitmentHash: BN;
  gameAmount: BN;
  userPubkey: BN;
};

class Game {
  gameParameters: GameParameters;
  programUtxo: Utxo;
  pda: PublicKey;

  constructor(
    gameParameters: GameParameters,
    programUtxo: Utxo,
    pda: PublicKey,
  ) {
    this.gameParameters = gameParameters;
    this.programUtxo = programUtxo;
    this.pda = pda;
  }

  static generateGameCommitmentHash(
    provider: LightProvider,
    gameParameters: GameParameters,
  ) {
    return new BN(
      provider.hasher.poseidonHashString([
        new BN(gameParameters.choice),
        gameParameters.slot,
        gameParameters.player2CommitmentHash,
        gameParameters.gameAmount,
      ]),
    );
  }

  static async create(
    choice: Choice,
    gameAmount: BN,
    lightProvider: LightProvider,
  ) {
    const slot = await lightProvider.connection.getSlot();
    const gameParameters: GameParameters = {
      choice,
      slot: new BN(slot),
      gameAmount,
      player2CommitmentHash: BN_0,
      userPubkey: BN_0,
    };
    gameParameters.gameCommitmentHash = Game.generateGameCommitmentHash(
      lightProvider,
      gameParameters,
    );
    const programUtxo = new Utxo({
      hasher: HASHER,
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      assets: [SystemProgram.programId],
      amounts: [gameParameters.gameAmount],
      appData: {
        gameCommitmentHash: gameParameters.gameCommitmentHash,
        userPubkey: gameParameters.userPubkey,
      },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    let seed = gameParameters.gameCommitmentHash.toArray("le", 32);
    const pda = findProgramAddressSync(
      [Buffer.from(seed)],
      new PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"),
    )[0];

    return new Game(gameParameters, programUtxo, pda);
  }

  static async join(
    gameCommitmentHash: BN,
    choice: Choice,
    gameAmount: BN,
    lightProvider: LightProvider,
    account: Account,
  ) {
    const slot = await lightProvider.connection.getSlot();
    const gameParameters: GameParameters = {
      choice,
      slot: new BN(slot),
      gameAmount,
      player2CommitmentHash: gameCommitmentHash,
      userPubkey: account.keypair.publicKey,
    };
    gameParameters.gameCommitmentHash = Game.generateGameCommitmentHash(
      lightProvider,
      gameParameters,
    );

    const programUtxo = new Utxo({
      hasher: lightProvider.hasher,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      amounts: [gameAmount],
      appData: {
        gameCommitmentHash: gameParameters.gameCommitmentHash,
        userPubkey: account.keypair.publicKey,
      },
      appDataIdl: IDL,
      verifierAddress: verifierProgramId,
      assetLookupTable: lightProvider.lookUpTables.assetLookupTable,
    });

    let seed = gameCommitmentHash.toArray("le", 32);
    const pda = findProgramAddressSync(
      [Buffer.from(seed)],
      new PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"),
    )[0];
    return new Game(gameParameters, programUtxo, pda);
  }

  getWinner(opponentChoice: Choice): {
    winner: Winner;
    isWin: BN[];
    isDraw: BN;
    isLoss: BN;
  } {
    const { choice } = this.gameParameters;
    if (choice === opponentChoice) {
      return {
        winner: Winner.DRAW,
        isWin: [BN_0, BN_0, BN_0],
        isDraw: BN_1,
        isLoss: BN_0,
      };
    }
    if (choice === Choice.ROCK && opponentChoice === Choice.SCISSORS) {
      return {
        winner: Winner.PLAYER1,
        isWin: [BN_1, BN_0, BN_0],
        isDraw: BN_0,
        isLoss: BN_0,
      };
    }
    if (choice === Choice.PAPER && opponentChoice === Choice.ROCK) {
      return {
        winner: Winner.PLAYER1,
        isWin: [BN_0, BN_1, BN_0],
        isDraw: BN_0,
        isLoss: BN_0,
      };
    }
    if (choice === Choice.SCISSORS && opponentChoice === Choice.PAPER) {
      return {
        winner: Winner.PLAYER1,
        isWin: [BN_0, BN_0, BN_1],
        isDraw: BN_0,
        isLoss: BN_0,
      };
    }
    return {
      winner: Winner.PLAYER2,
      isWin: [BN_0, BN_0, BN_0],
      isDraw: BN_0,
      isLoss: BN_1,
    };
  }
}

class Player {
  user: User;
  game?: Game;
  pspInstance: anchor.Program<RockPaperScissors>;

  constructor(user: User) {
    this.user = user;
    this.pspInstance = new anchor.Program(
      IDL,
      new PublicKey("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"),
      user.provider.provider,
    );
  }

  static async init(
    provider: anchor.AnchorProvider,
    relayer: TestRelayer | Relayer,
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
    return new Player(await User.init({ provider: lightProvider }));
  }

  async closeGame() {
    if (!this.game) {
      throw new Error("No game in progress.");
    }
    let tx = await this.pspInstance.methods
      .closeGame()
      .accounts({
        gamePda: this.game.pda,
        signer: this.user.provider.wallet.publicKey,
      })
      .instruction();

    await sendVersionedTransactions(
      [tx],
      this.user.provider.connection,
      this.user.provider.lookUpTables.versionedTransactionLookupTable,
      this.user.provider.wallet,
    );
  }
  async createGame(
    choice: Choice,
    gameAmount: BN,
    action: Action = Action.SHIELD,
  ) {
    if (this.game) {
      throw new Error("A game is already in progress.");
    }
    this.game = await Game.create(choice, gameAmount, this.user.provider);

    const txHash = await this.user.storeAppUtxo({
      appUtxo: this.game.programUtxo,
      action,
    });

    const borshCoder = new anchor.BorshAccountsCoder(IDL);
    const serializationObject = {
      ...this.game.programUtxo,
      ...this.game.programUtxo.appData,
      accountEncryptionPublicKey: this.game.programUtxo.encryptionPublicKey,
      accountShieldedPublicKey: this.game.programUtxo.publicKey,
    };
    const utxoBytes = (
      await borshCoder.encode("utxo", serializationObject)
    ).subarray(8);

    let tx = await this.pspInstance.methods
      .createGame(utxoBytes)
      .accounts({
        gamePda: this.game.pda,
        signer: this.user.provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .instruction();

    let txHash2 = await sendVersionedTransactions(
      [tx],
      this.user.provider.connection,
      this.user.provider.lookUpTables.versionedTransactionLookupTable,
      this.user.provider.wallet,
    );

    return {
      game: this.game,
      txHashStoreAppUtxo: txHash,
      txHashCreateGame: txHash2,
    };
  }

  async join(
    gameCommitmentHash: BN,
    choice: Choice,
    gameAmount: BN,
    action: Action = Action.SHIELD,
  ) {
    if (this.game) {
      throw new Error("A game is already in progress.");
    }
    this.game = await Game.join(
      gameCommitmentHash,
      choice,
      gameAmount,
      this.user.provider,
      this.user.account,
    );
    const txHash = await this.user.storeAppUtxo({
      appUtxo: this.game.programUtxo,
      action,
    });
    const gamePdaAccountInfo = await this.pspInstance.account.gamePda.fetch(
      this.game.pda,
    );
    // @ts-ignore anchor type is not represented correctly
    if (gamePdaAccountInfo.game.isJoinable === false) {
      throw new Error("Game is not joinable");
    }

    const borshCoder = new anchor.BorshAccountsCoder(IDL);
    const serializationObject = {
      ...this.game.programUtxo,
      ...this.game.programUtxo.appData,
      accountEncryptionPublicKey: this.game.programUtxo.encryptionPublicKey,
      accountShieldedPublicKey: this.game.programUtxo.publicKey,
    };
    const utxoBytes = (
      await borshCoder.encode("utxo", serializationObject)
    ).subarray(8);

    const tx = await this.pspInstance.methods
      .joinGame(
        utxoBytes,
        this.game.gameParameters.choice,
        this.game.gameParameters.slot,
      )
      .accounts({
        gamePda: this.game.pda,
        signer: this.user.provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .instruction();
    let txHash2 = await sendVersionedTransactions(
      [tx],
      this.user.provider.connection,
      this.user.provider.lookUpTables.versionedTransactionLookupTable,
      this.user.provider.wallet,
    );

    return {
      game: this.game,
      txHashStoreAppUtxo: txHash,
      txHashCreateGame: txHash2,
    };
  }

  async execute(testProgramUtxo?: Utxo) {
    const gamePdaAccountInfo = await this.pspInstance.account.gamePda.fetch(
      this.game.pda,
    );
    if (gamePdaAccountInfo.game.isJoinable === true) {
      throw new Error("Game is joinable not executable");
    }
    const gameParametersPlayer2 = {
      gameCommitmentHash:
        gamePdaAccountInfo.game.playerTwoProgramUtxo.gameCommitmentHash,
      choice: gamePdaAccountInfo.game.playerTwoChoice,
      slot: gamePdaAccountInfo.game.slot,
      userPubkey: gamePdaAccountInfo.game.playerTwoProgramUtxo.userPubkey,
    };
    const player2ProgramUtxo = new Utxo({
      hasher: this.user.provider.hasher,
      assets: [SystemProgram.programId],
      publicKey: STANDARD_SHIELDED_PUBLIC_KEY,
      amounts: [gamePdaAccountInfo.game.playerTwoProgramUtxo.amounts[0]],
      appData: {
        gameCommitmentHash: gameParametersPlayer2.gameCommitmentHash,
        userPubkey: gameParametersPlayer2.userPubkey,
      },
      appDataIdl: IDL,
      verifierAddress: new PublicKey(
        "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS",
      ),
      assetLookupTable: this.user.provider.lookUpTables.assetLookupTable,
      blinding: gamePdaAccountInfo.game.playerTwoProgramUtxo.blinding,
    });
    Utxo.equal(
      this.user.provider.hasher,
      player2ProgramUtxo,
      testProgramUtxo,
      false,
    );
    const circuitPath = path.join(
      "build-circuit/rock-paper-scissors/rockPaperScissors",
    );
    const winner = this.game.getWinner(gamePdaAccountInfo.game.playerTwoChoice);

    // We use getBalance to sync the current merkle tree
    await this.user.getBalance();
    const merkleTree = this.user.provider.solMerkleTree.merkleTree;
    this.game.programUtxo.index = merkleTree.indexOf(
      this.game.programUtxo.getCommitment(this.user.provider.hasher),
    );
    player2ProgramUtxo.index = merkleTree.indexOf(
      player2ProgramUtxo.getCommitment(this.user.provider.hasher),
    );

    const programParameters: ProgramParameters = {
      inputs: {
        publicGameCommitment0: this.game.gameParameters.gameCommitmentHash,
        publicGameCommitment1: player2ProgramUtxo.appData.gameCommitmentHash,
        gameCommitmentHash: [
          this.game.gameParameters.gameCommitmentHash,
          gameParametersPlayer2.gameCommitmentHash,
        ],
        choice: [this.game.gameParameters.choice, gameParametersPlayer2.choice],
        slot: [this.game.gameParameters.slot, gameParametersPlayer2.slot],
        gameAmount: GAME_AMOUNT,
        userPubkey: [
          this.game.gameParameters.userPubkey,
          player2ProgramUtxo.appData.userPubkey,
        ],
        isPlayer2OutUtxo: [[BN_0, BN_1, BN_0, BN_0]],
        ...winner,
      },
      verifierIdl: IDL,
      path: circuitPath,
      accounts: {
        gamePda: this.game.pda,
      },
      circuitName: "rockPaperScissors",
    };
    const amounts = this.getAmounts(winner.winner);
    const player1OutUtxo = new Utxo({
      hasher: this.user.provider.hasher,
      assets: [SystemProgram.programId],
      publicKey: this.user.account.pubkey,
      amounts: [amounts[0]],
      assetLookupTable: this.user.provider.lookUpTables.assetLookupTable,
    });
    const player2OutUtxo = new Utxo({
      hasher: this.user.provider.hasher,
      assets: [SystemProgram.programId],
      publicKey: gameParametersPlayer2.userPubkey,
      encryptionPublicKey: new Uint8Array(
        gamePdaAccountInfo.game.playerTwoProgramUtxo.accountEncryptionPublicKey,
      ),
      amounts: [amounts[1]],
      assetLookupTable: this.user.provider.lookUpTables.assetLookupTable,
      blinding: gameParametersPlayer2.userPubkey
        .add(gameParametersPlayer2.userPubkey)
        .mod(FIELD_SIZE),
    });

    let payerUtxo = this.user.getAllUtxos();

    let { txHash } = await this.user.executeAppUtxo({
      appUtxos: [this.game.programUtxo, player2ProgramUtxo],
      inUtxos: [payerUtxo[0]],
      outUtxos: [player1OutUtxo, player2OutUtxo],
      programParameters,
      action: Action.TRANSFER,
      addOutUtxos: true,
      shuffleEnabled: false,
      confirmOptions: ConfirmOptions.spendable,
    });

    return { txHash, gameResult: winner.winner };
  }

  getAmounts(winner: Winner) {
    if (winner === Winner.PLAYER1) {
      return [this.game.gameParameters.gameAmount.mul(new BN(2)), BN_0];
    } else if (winner === Winner.PLAYER2) {
      return [BN_0, this.game.gameParameters.gameAmount.mul(new BN(2))];
    }
    return [
      this.game.gameParameters.gameAmount,
      this.game.gameParameters.gameAmount,
    ];
  }
}

describe("Test rock-paper-scissors", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(RPC_URL, confirmConfig);
  anchor.setProvider(provider);

  before(async () => {
    HASHER = await WasmHasher.getInstance();

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

  it.skip("Test Game Draw", async () => {
    const player1 = await Player.init(provider, RELAYER);
    // shield additional sol to pay for relayer fees
    await player1.user.shield({
      publicAmountSol: 10,
      token: "SOL",
    });
    const player2 = await Player.init(provider, RELAYER);

    let res = await player1.createGame(Choice.ROCK, GAME_AMOUNT);
    console.log("Player 1 created game");
    await player2.join(
      res.game.gameParameters.gameCommitmentHash,
      Choice.ROCK,
      GAME_AMOUNT,
    );
    console.log("Player 2 joined game");
    let gameRes = await player1.execute(player2.game.programUtxo);
    console.log("Game result: ", gameRes.gameResult);
    assert.equal(gameRes.gameResult, Winner.DRAW);
    await player1.closeGame();
  });

  it.skip("Test Game Loss", async () => {
    const player1 = await Player.init(provider, RELAYER);
    // shield additional sol to pay for relayer fees
    await player1.user.shield({
      publicAmountSol: 10,
      token: "SOL",
    });
    const player2 = await Player.init(provider, RELAYER);

    let res = await player1.createGame(Choice.SCISSORS, GAME_AMOUNT);
    console.log("Player 1 created game");
    await player2.join(
      res.game.gameParameters.gameCommitmentHash,
      Choice.ROCK,
      GAME_AMOUNT,
    );
    console.log("Player 2 joined game");
    let gameRes = await player1.execute(player2.game.programUtxo);
    console.log("Game result: ", gameRes.gameResult);
    assert.equal(gameRes.gameResult, Winner.PLAYER2);
    await player1.closeGame();
  });

  it.skip("Test Game Win", async () => {
    const player1 = await Player.init(provider, RELAYER);
    // shield additional sol to pay for relayer fees
    await player1.user.shield({
      publicAmountSol: 10,
      token: "SOL",
    });
    const player2 = await Player.init(provider, RELAYER);

    let res = await player1.createGame(Choice.PAPER, GAME_AMOUNT);
    console.log("Player 1 created game");
    await player2.join(
      res.game.gameParameters.gameCommitmentHash,
      Choice.ROCK,
      GAME_AMOUNT,
    );
    console.log("Player 2 joined game");
    let gameRes = await player1.execute(player2.game.programUtxo);
    console.log("Game result: ", gameRes.gameResult);
    assert.equal(gameRes.gameResult, Winner.PLAYER1);
    await player1.closeGame();
  });
});
