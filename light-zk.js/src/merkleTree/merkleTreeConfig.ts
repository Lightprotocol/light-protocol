import * as anchor from "@coral-xyz/anchor";
import { IDL_MERKLE_TREE_PROGRAM, MerkleTreeProgram } from "../idls/index";
import { assert } from "chai";
const token = require("@solana/spl-token");
import {
  Connection,
  PublicKey,
  Keypair,
  sendAndConfirmTransaction,
} from "@solana/web3.js";

import {
  BN_0,
  BN_1,
  confirmConfig,
  DEFAULT_PROGRAMS,
  merkleTreeProgramId,
} from "../index";
import { Program } from "@coral-xyz/anchor";

export class MerkleTreeConfig {
  merkleTreeProgram: Program<MerkleTreeProgram>;
  transactionMerkleTreePda?: PublicKey;
  connection: Connection;
  registeredVerifierPdas: any;
  preInsertedLeavesIndex?: PublicKey;
  merkleTreeAuthorityPda?: PublicKey;
  payer?: Keypair;
  tokenAuthority?: PublicKey;
  // TODO: save state effectively
  poolTypes: {
    tokenPdas: { mint: PublicKey; pubkey: PublicKey }[];
    poolPda: PublicKey;
    poolType: Array<number>;
  }[];
  poolPdas: any;
  constructor({
    payer,
    connection,
  }: {
    payer?: Keypair;
    connection: Connection;
  }) {
    this.payer = payer;
    this.merkleTreeProgram = new Program(
      IDL_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
    );
    // TODO: reorg pool pdas, have one object per pool type and then an array with registered pools of this type
    this.poolPdas = [];
    this.poolTypes = [];
    this.registeredVerifierPdas = [];
    this.connection = connection;
  }

  async initializeNewMerkleTrees() {
    if (!this.payer) throw new Error("Payer undefined");
    this.getMerkleTreeAuthorityPda();

    let merkleTreeAuthorityAccountInfo =
      await this.getMerkleTreeAuthorityAccountInfo();
    const transactionMerkleTreeIndex =
      merkleTreeAuthorityAccountInfo.transactionMerkleTreeIndex;
    const eventMerkleTreeIndex =
      merkleTreeAuthorityAccountInfo.eventMerkleTreeIndex;

    const oldTransactionMerkleTree =
      MerkleTreeConfig.getTransactionMerkleTreePda(
        transactionMerkleTreeIndex.sub(BN_1),
      );
    const newTransactionMerkleTree =
      MerkleTreeConfig.getTransactionMerkleTreePda(transactionMerkleTreeIndex);
    const oldEventMerkleTree = MerkleTreeConfig.getEventMerkleTreePda(
      eventMerkleTreeIndex.sub(BN_1),
    );
    const newEventMerkleTree =
      MerkleTreeConfig.getEventMerkleTreePda(eventMerkleTreeIndex);

    const tx = await this.merkleTreeProgram.methods
      .initializeNewMerkleTrees(new anchor.BN("50"))
      .accounts({
        authority: this.payer.publicKey,
        newTransactionMerkleTree: newTransactionMerkleTree,
        newEventMerkleTree: newEventMerkleTree,
        systemProgram: DEFAULT_PROGRAMS.systemProgram,
        rent: DEFAULT_PROGRAMS.rent,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
      })
      .remainingAccounts([
        {
          isSigner: false,
          isWritable: true,
          pubkey: oldTransactionMerkleTree,
        },
        {
          isSigner: false,
          isWritable: true,
          pubkey: oldEventMerkleTree,
        },
      ])
      .signers([this.payer])
      .transaction();

    const txHash = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [this.payer!],
      confirmConfig,
    );

    await this.checkTransactionMerkleTreeIsInitialized(
      newTransactionMerkleTree,
    );
    await this.checkEventMerkleTreeIsInitialized(newEventMerkleTree);
    return txHash;
  }

  async checkTransactionMerkleTreeIsInitialized(
    transactionMerkleTreePda: PublicKey,
  ) {
    var transactionMerkleTreeAccountInfo =
      await this.merkleTreeProgram.account.transactionMerkleTree.fetch(
        transactionMerkleTreePda,
      );
    assert(
      transactionMerkleTreeAccountInfo != null,
      "merkleTreeAccountInfo not initialized",
    );
    // zero values
    // index == 0
    // roots are empty save for 0
    // lock duration is correct
    assert(transactionMerkleTreeAccountInfo.lockDuration.toString() == "50");
  }

  async checkEventMerkleTreeIsInitialized(eventMerkleTreePubkey: PublicKey) {
    var merkleTreeAccountInfo =
      await this.merkleTreeProgram.account.eventMerkleTree.fetch(
        eventMerkleTreePubkey,
      );
    assert(
      merkleTreeAccountInfo != null,
      "merkleTreeAccountInfo not initialized",
    );
  }

  async printMerkleTree() {
    var merkleTreeAccountInfo =
      await this.merkleTreeProgram.account.transactionMerkleTree.fetch(
        MerkleTreeConfig.getTransactionMerkleTreePda(),
      );
    console.log("merkleTreeAccountInfo ", merkleTreeAccountInfo);
  }

  getMerkleTreeAuthorityPda() {
    this.merkleTreeAuthorityPda = PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY")],
      this.merkleTreeProgram.programId,
    )[0];
    return this.merkleTreeAuthorityPda;
  }

  async getMerkleTreeAuthorityAccountInfo() {
    return await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
      this.getMerkleTreeAuthorityPda(),
    );
  }

  async getTransactionMerkleTreeIndex(): Promise<anchor.BN> {
    let merkleTreeAuthorityAccountInfo =
      await this.getMerkleTreeAuthorityAccountInfo();
    return merkleTreeAuthorityAccountInfo.transactionMerkleTreeIndex;
  }

  static getTransactionMerkleTreePda(
    transactionMerkleTreeIndex: anchor.BN = new anchor.BN(0),
  ) {
    let transactionMerkleTreePda = PublicKey.findProgramAddressSync(
      [
        anchor.utils.bytes.utf8.encode("transaction_merkle_tree"),
        transactionMerkleTreeIndex.toArrayLike(Buffer, "le", 8),
      ],
      merkleTreeProgramId,
    )[0];
    return transactionMerkleTreePda;
  }

  static getEventMerkleTreePda(eventMerkleTreeIndex: anchor.BN = BN_0) {
    let eventMerkleTreePda = PublicKey.findProgramAddressSync(
      [
        anchor.utils.bytes.utf8.encode("event_merkle_tree"),
        eventMerkleTreeIndex.toArrayLike(Buffer, "le", 8),
      ],
      merkleTreeProgramId,
    )[0];
    return eventMerkleTreePda;
  }

  async initMerkleTreeAuthority(
    authority?: Keypair | undefined,
    transactionMerkleTree?: PublicKey,
    eventMerkleTree?: PublicKey,
  ) {
    if (authority == undefined) {
      authority = this.payer;
    }
    if (transactionMerkleTree == undefined) {
      transactionMerkleTree =
        MerkleTreeConfig.getTransactionMerkleTreePda(BN_0);
    }
    if (eventMerkleTree === undefined) {
      eventMerkleTree = MerkleTreeConfig.getEventMerkleTreePda(BN_0);
    }
    if (this.merkleTreeAuthorityPda == undefined) {
      await this.getMerkleTreeAuthorityPda();
    }

    const tx = await this.merkleTreeProgram.methods
      .initializeMerkleTreeAuthority()
      .accounts({
        authority: authority?.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        transactionMerkleTree: transactionMerkleTree,
        eventMerkleTree: eventMerkleTree,
        ...DEFAULT_PROGRAMS,
      })
      .signers([authority!])
      .transaction();

    const txHash = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [authority ? authority : this.payer!],
      confirmConfig,
    );

    // assert(
    //   this.connection.getAccountInfo(
    //     this.merkleTreeAuthorityPda!,
    //     "confirmed",
    //   ) != null,
    //   "init authority failed",
    // );
    // let merkleTreeAuthority =
    //   await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
    //     this.merkleTreeAuthorityPda!,
    //   );
    // assert(merkleTreeAuthority.enablePermissionlessSplTokens == false);
    // assert(merkleTreeAuthority.enableNfts == false);
    // assert(
    //   merkleTreeAuthority.pubkey.toBase58() == authority!.publicKey.toBase58(),
    // );
    // assert(merkleTreeAuthority.registeredAssetIndex.toString() == "0");

    return txHash;
  }

  async isMerkleTreeAuthorityInitialized(): Promise<boolean> {
    const accountInfo = await this.connection.getAccountInfo(
      this.getMerkleTreeAuthorityPda(),
    );

    return accountInfo !== null && accountInfo.data.length >= 0;
  }

  async updateMerkleTreeAuthority(newAuthority: PublicKey, test = false) {
    if (!this.merkleTreeAuthorityPda) {
      await this.getMerkleTreeAuthorityPda();
    }
    if (!this.payer) throw new Error("Payer undefined");

    let merkleTreeAuthorityPrior: any = null;
    if (test != true) {
      merkleTreeAuthorityPrior =
        await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
          this.merkleTreeAuthorityPda!,
        );
      if (merkleTreeAuthorityPrior == null) {
        throw `Merkle tree authority ${this.merkleTreeAuthorityPda!.toBase58()} not initialized`;
      }
    }

    const tx = await this.merkleTreeProgram.methods
      .updateMerkleTreeAuthority()
      .accounts({
        authority: this.payer.publicKey,
        newAuthority,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS,
      })
      .signers([this.payer])
      .transaction();

    const txHash = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [this.payer!],
      confirmConfig,
    );

    if (test != true) {
      let merkleTreeAuthority =
        await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
          this.merkleTreeAuthorityPda!,
        );

      assert.equal(
        merkleTreeAuthority.enablePermissionlessSplTokens,
        merkleTreeAuthorityPrior!.enablePermissionlessSplTokens,
      );
      assert.equal(
        merkleTreeAuthority.enableNfts,
        merkleTreeAuthorityPrior!.enableNfts,
      );
      assert.equal(
        merkleTreeAuthority.pubkey.toBase58(),
        newAuthority.toBase58(),
      );
    }
    return txHash;
  }

  // commented in program
  // async enableNfts(configValue: Boolean) {
  //   if (this.merkleTreeAuthorityPda == undefined) {
  //     await this.getMerkleTreeAuthorityPda();
  //   }
  //   const tx = await this.merkleTreeProgram.methods
  //     .enableNfts(configValue)
  //     .accounts({
  //       authority: this.payer.publicKey,
  //       merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
  //       ...DEFAULT_PROGRAMS,
  //     })
  //     .signers([this.payer])
  //     .rpc(confirmConfig);
  //   let merkleTreeAuthority =
  //     await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
  //       this.merkleTreeAuthorityPda,
  //     );
  //   assert(merkleTreeAuthority.enableNfts == configValue);

  //   return tx;
  // }

  async enablePermissionlessSplTokens(configValue: boolean) {
    if (!this.payer) throw new Error("Payer undefined");
    if (this.merkleTreeAuthorityPda == undefined) {
      await this.getMerkleTreeAuthorityPda();
    }
    const tx = await this.merkleTreeProgram.methods
      .enablePermissionlessSplTokens(configValue)
      .accounts({
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS,
      })
      .signers([this.payer])
      .transaction();

    const txHash = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [this.payer!],
      confirmConfig,
    );
    let merkleTreeAuthority =
      await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
        this.merkleTreeAuthorityPda!,
      );
    assert(merkleTreeAuthority.enablePermissionlessSplTokens == configValue);
    return txHash;
  }

  async updateLockDuration(lockDuration: Number) {
    if (!this.payer) throw new Error("Payer undefined");
    if (this.merkleTreeAuthorityPda == undefined) {
      await this.getMerkleTreeAuthorityPda();
    }

    const transactionMerkleTreePda =
      MerkleTreeConfig.getTransactionMerkleTreePda();

    const tx = await this.merkleTreeProgram.methods
      .updateLockDuration(new anchor.BN(lockDuration.toString()))
      .accounts({
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        transactionMerkleTree: transactionMerkleTreePda,
        ...DEFAULT_PROGRAMS,
      })
      .signers([this.payer])
      .transaction();

    const txHash = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [this.payer!],
      confirmConfig,
    );
    let merkleTree =
      await this.merkleTreeProgram.account.transactionMerkleTree.fetch(
        transactionMerkleTreePda!,
      );
    assert.equal(merkleTree.lockDuration.toString(), lockDuration.toString());
    console.log("lock duration updated to: ", lockDuration);

    return txHash;
  }

  async getRegisteredVerifierPda(verifierPubkey: PublicKey) {
    // TODO: add check whether already exists
    this.registeredVerifierPdas.push({
      registeredVerifierPda: PublicKey.findProgramAddressSync(
        [verifierPubkey.toBuffer()],
        this.merkleTreeProgram.programId,
      )[0],
      verifierPubkey: verifierPubkey,
    });
    return this.registeredVerifierPdas[this.registeredVerifierPdas.length - 1];
  }

  async registerVerifier(verifierPubkey: PublicKey) {
    if (!this.payer) throw new Error("Payer undefined");

    let registeredVerifierPda = this.registeredVerifierPdas.filter(
      (item: any) => {
        return item.verifierPubkey === verifierPubkey;
      },
    )[0];

    if (!registeredVerifierPda) {
      registeredVerifierPda = await this.getRegisteredVerifierPda(
        verifierPubkey,
      );
    }

    const tx = await this.merkleTreeProgram.methods
      .registerVerifier(verifierPubkey)
      .accounts({
        registeredVerifierPda: registeredVerifierPda.registeredVerifierPda,
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS,
      })
      .signers([this.payer])
      .transaction();

    const txHash = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [this.payer!],
      confirmConfig,
    );

    await this.checkVerifierIsRegistered(verifierPubkey);

    return txHash;
  }

  async checkVerifierIsRegistered(verifierPubkey: PublicKey) {
    let registeredVerifierPda = this.registeredVerifierPdas.filter(
      (item: any) => {
        return item.verifierPubkey === verifierPubkey;
      },
    )[0];

    var registeredVerifierAccountInfo =
      await this.merkleTreeProgram.account.registeredVerifier.fetch(
        registeredVerifierPda.registeredVerifierPda,
      );

    assert(registeredVerifierAccountInfo != null);
    assert(
      registeredVerifierAccountInfo.pubkey.toBase58() ==
        verifierPubkey.toBase58(),
    );
  }

  async getPoolTypePda(poolType: Array<number>) {
    if (poolType.length != 32) {
      throw `invalid pooltype length ${poolType.length}`;
    }
    // TODO: add check whether already exists

    this.poolTypes.push({
      tokenPdas: [],
      poolPda: PublicKey.findProgramAddressSync(
        [Buffer.from(poolType), anchor.utils.bytes.utf8.encode("pooltype")],
        this.merkleTreeProgram.programId,
      )[0],
      poolType: poolType,
    });
    return this.poolTypes[this.poolTypes.length - 1];
  }

  async registerPoolType(poolType: Array<number>) {
    if (!this.payer) throw new Error("Payer undefined");

    let registeredPoolTypePda = this.poolTypes.filter((item) => {
      return item.poolType.toString() === poolType.toString();
    })[0];

    if (!registeredPoolTypePda) {
      registeredPoolTypePda = await this.getPoolTypePda(poolType);
    }

    const tx = await this.merkleTreeProgram.methods
      .registerPoolType(poolType)
      .accounts({
        registeredPoolTypePda: registeredPoolTypePda.poolPda,
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS,
      })
      .signers([this.payer])
      .transaction();

    const txHash = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [this.payer!],
      confirmConfig,
    );
    return txHash;
  }

  async checkPoolRegistered(
    poolPda: any,
    poolType: Array<number>,
    mint: PublicKey | null = null,
  ) {
    if (!this.merkleTreeAuthorityPda)
      throw new Error("merkleTreeAuthorityPda undefined");
    var registeredTokenConfigAccount =
      await this.merkleTreeProgram.account.registeredAssetPool.fetch(
        poolPda.pda,
      );

    var merkleTreeAuthorityPdaAccountInfo =
      await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
        this.merkleTreeAuthorityPda,
      );
    assert.equal(
      registeredTokenConfigAccount.poolType.toString(),
      poolType.toString(),
    );
    assert.equal(
      registeredTokenConfigAccount.index.toString(),
      (
        merkleTreeAuthorityPdaAccountInfo.registeredAssetIndex.toNumber() - 1
      ).toString(),
    );

    if (mint !== null) {
      assert.equal(
        registeredTokenConfigAccount.assetPoolPubkey.toBase58(),
        poolPda.token.toBase58(),
      );

      var registeredTokenAccount = await token.getAccount(
        this.connection,
        poolPda.token,
        { commitment: "confirmed", preflightCommitment: "confirmed" },
      );
      assert.notEqual(registeredTokenAccount, null);
      assert.equal(registeredTokenAccount.mint.toBase58(), mint.toBase58());
    } else {
      assert.equal(
        registeredTokenConfigAccount.assetPoolPubkey.toBase58(),
        poolPda.pda.toBase58(),
      );
    }
  }

  static getSolPoolPda(
    programId: PublicKey,
    poolType: Array<number> = new Array(32),
  ) {
    return {
      pda: PublicKey.findProgramAddressSync(
        [
          new Uint8Array(32).fill(0),
          Buffer.from(poolType),
          anchor.utils.bytes.utf8.encode("pool-config"),
        ],
        programId,
      )[0],
      poolType: poolType,
    };
  }

  async registerSolPool(poolType: Array<number>) {
    if (!this.payer) throw new Error("Payer undefined");
    if (!this.merkleTreeAuthorityPda)
      throw new Error("merkleTreeAuthorityPda undefined");

    let registeredPoolTypePda = this.poolTypes.filter((item) => {
      return item.poolType.toString() === poolType.toString();
    })[0];

    if (!registeredPoolTypePda) {
      registeredPoolTypePda = await this.getPoolTypePda(poolType);
    }
    let solPoolPda = MerkleTreeConfig.getSolPoolPda(
      this.merkleTreeProgram.programId,
      poolType,
    );

    const tx = await this.merkleTreeProgram.methods
      .registerSolPool()
      .accounts({
        registeredAssetPoolPda: solPoolPda.pda,
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        registeredPoolTypePda: registeredPoolTypePda.poolPda,
        ...DEFAULT_PROGRAMS,
      })
      .signers([this.payer])
      .transaction();

    const txHash = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [this.payer!],
      confirmConfig,
    );

    await this.checkPoolRegistered(solPoolPda, poolType);
    console.log("registered sol pool ", this.merkleTreeAuthorityPda.toBase58());
    // no need to push the sol pool because it is the pool config pda
    // TODO: evaluate how to handle this case
    return txHash;
  }

  static getSplPoolPdaToken(
    mint: PublicKey,
    programId: PublicKey,
    poolType: Array<number> = new Array(32).fill(0),
  ) {
    let pda = PublicKey.findProgramAddressSync(
      [
        mint.toBytes(),
        Buffer.from(poolType),
        anchor.utils.bytes.utf8.encode("pool"),
      ],
      programId,
    )[0];
    return pda;
  }

  async getSplPoolPda(
    mint: PublicKey,
    poolType: Array<number> = new Array(32).fill(0),
  ) {
    this.poolPdas.push({
      pda: PublicKey.findProgramAddressSync(
        [
          mint.toBytes(),
          new Uint8Array(32).fill(0),
          anchor.utils.bytes.utf8.encode("pool-config"),
        ],
        this.merkleTreeProgram.programId,
      )[0],
      poolType: poolType,
      token: await MerkleTreeConfig.getSplPoolPdaToken(
        mint,
        this.merkleTreeProgram.programId,
        poolType,
      ),
    });
    return this.poolPdas[this.poolPdas.length - 1];
  }

  async getTokenAuthority() {
    this.tokenAuthority = PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("spl")],
      this.merkleTreeProgram.programId,
    )[0];
    return this.tokenAuthority;
  }

  async registerSplPool(poolType: Array<number>, mint: PublicKey) {
    if (!this.payer) throw new Error("Payer undefined");
    let registeredPoolTypePda = this.poolTypes.filter((item) => {
      return item.poolType === poolType;
    })[0];

    if (!registeredPoolTypePda) {
      registeredPoolTypePda = await this.getPoolTypePda(poolType);
    }

    let splPoolPda = await this.getSplPoolPda(mint, poolType);

    if (!this.tokenAuthority) {
      await this.getTokenAuthority();
    }

    const tx = await this.merkleTreeProgram.methods
      .registerSplPool()
      .accounts({
        registeredAssetPoolPda: splPoolPda.pda,
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        registeredPoolTypePda: registeredPoolTypePda.poolPda,
        merkleTreePdaToken: splPoolPda.token,
        tokenAuthority: this.tokenAuthority,
        mint,
        ...DEFAULT_PROGRAMS,
      })
      .signers([this.payer])
      .transaction();

    const txHash = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [this.payer!],
      confirmConfig,
    );

    await this.checkPoolRegistered(splPoolPda, poolType, mint);

    return txHash;
  }
}
