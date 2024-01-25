import { assert } from "chai";
import { Program, BN, AnchorProvider } from "@coral-xyz/anchor";
import { utf8 } from "@coral-xyz/anchor/dist/cjs/utils/bytes";
import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import * as token from "@solana/spl-token";
import { IDL_LIGHT_MERKLE_TREE_PROGRAM, LightMerkleTreeProgram } from "../idls";
import {
  BN_0,
  BN_1,
  DEFAULT_PROGRAMS,
  confirmConfig,
  merkleTreeProgramId,
} from "../constants";

/// NODE ENV ONLY
export class MerkleTreeConfig {
  merkleTreeProgram: Program<LightMerkleTreeProgram>;
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
    anchorProvider,
  }: {
    payer?: Keypair;
    anchorProvider: AnchorProvider;
  }) {
    this.payer = payer;

    this.merkleTreeProgram = new Program(
      IDL_LIGHT_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      anchorProvider,
    );

    // TODO: reorg pool pdas, have one object per pool type and then an array with registered pools of this type
    this.poolPdas = [];
    this.poolTypes = [];
    this.registeredVerifierPdas = [];
    this.connection = anchorProvider.connection;
  }

  async initializeNewMerkleTreeSet(merkleTreeSet: Keypair) {
    if (!this.payer) throw new Error("Payer undefined");
    this.getMerkleTreeAuthorityPda();

    const merkleTreeAuthorityAccountInfo =
      await this.getMerkleTreeAuthorityAccountInfo();
    const merkleTreeSetIndex =
      merkleTreeAuthorityAccountInfo.merkleTreeSetIndex;

    // TODO(vadorovsky): Expose account sizes through a WASM shim.
    const space = 180752;
    const ix1 = SystemProgram.createAccount({
      fromPubkey: this.payer!.publicKey,
      newAccountPubkey: merkleTreeSet.publicKey,
      space,
      lamports: await this.connection.getMinimumBalanceForRentExemption(space),
      programId: merkleTreeProgramId,
    });
    const ix2 = await this.merkleTreeProgram.methods
      .initializeNewMerkleTreeSet()
      .accounts({
        authority: this.payer.publicKey,
        newMerkleTreeSet: merkleTreeSet.publicKey,
        systemProgram: DEFAULT_PROGRAMS.systemProgram,
        rent: DEFAULT_PROGRAMS.rent,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
      })
      .instruction();
    const tx = new Transaction().add(ix1, ix2);

    const txHash = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [this.payer!, merkleTreeSet],
      confirmConfig,
    );

    await this.checkMerkleTreeSetIsInitialized(
      merkleTreeSet.publicKey,
      merkleTreeSetIndex,
    );
    return {
      txHash,
      merkleTreeSet: merkleTreeSet.publicKey,
      index: merkleTreeSetIndex,
    };
  }

  async checkMerkleTreeSetIsInitialized(
    merkleTreeSetPubkey: PublicKey,
    merkleTreeSetIndex: BN,
  ) {
    const merkleTreeSetAccountInfo =
      await this.getMerkleTreeSetAccountInfo(merkleTreeSetPubkey);
    assert(
      merkleTreeSetAccountInfo != null,
      "merkleTreeAccountInfo not initialized",
    );
    assert.equal(
      merkleTreeSetIndex.toString(),
      merkleTreeSetAccountInfo.index.toString(),
      "Merkle tree number is not correct",
    );
    const merkleTreeAuthorityAccountInfo =
      await this.getMerkleTreeAuthorityAccountInfo();
    assert.equal(
      merkleTreeSetAccountInfo.index.toString(),
      merkleTreeAuthorityAccountInfo.merkleTreeSetIndex.sub(BN_1).toString(),
    );
  }

  getMerkleTreeAuthorityPda() {
    this.merkleTreeAuthorityPda = PublicKey.findProgramAddressSync(
      [utf8.encode("MERKLE_TREE_AUTHORITY")],
      this.merkleTreeProgram.programId,
    )[0];
    return this.merkleTreeAuthorityPda;
  }

  async getMerkleTreeAuthorityAccountInfo() {
    return await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
      this.getMerkleTreeAuthorityPda(),
    );
  }

  async getMerkleTreeSetIndex(): Promise<BN> {
    const merkleTreeAuthorityAccountInfo =
      await this.getMerkleTreeAuthorityAccountInfo();
    return merkleTreeAuthorityAccountInfo.merkleTreeSetIndex;
  }

  async getMerkleTreeSetAccountInfo(merkleTreeSetPubkey: PublicKey) {
    return await this.merkleTreeProgram.account.merkleTreeSet.fetch(
      merkleTreeSetPubkey,
    );
  }

  async initMerkleTreeAuthority({
    authority = this.payer,
    merkleTreeSet,
  }: {
    authority?: Keypair | undefined;
    merkleTreeSet: Keypair;
  }) {
    if (authority == undefined) {
      authority = this.payer;
    }
    if (this.merkleTreeAuthorityPda == undefined) {
      this.getMerkleTreeAuthorityPda();
    }

    // TODO(vadorovsky): Expose account sizes through a WASM shim.
    const space = 180752;
    const ix1 = SystemProgram.createAccount({
      fromPubkey: authority!.publicKey,
      newAccountPubkey: merkleTreeSet.publicKey,
      space,
      lamports: await this.connection.getMinimumBalanceForRentExemption(space),
      programId: merkleTreeProgramId,
    });
    const ix2 = await this.merkleTreeProgram.methods
      .initializeMerkleTreeAuthority()
      .accounts({
        authority: authority?.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        merkleTreeSet: merkleTreeSet.publicKey,
        ...DEFAULT_PROGRAMS,
      })
      .instruction();
    const tx = new Transaction().add(ix1, ix2);

    const txHash = await sendAndConfirmTransaction(
      this.connection,
      tx,
      [authority!, merkleTreeSet],
      confirmConfig,
    );

    assert(
      this.connection.getAccountInfo(
        this.merkleTreeAuthorityPda!,
        "confirmed",
      ) != null,
      "init authority failed",
    );
    const merkleTreeAuthority =
      await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
        this.merkleTreeAuthorityPda!,
      );
    assert(
      merkleTreeAuthority.pubkey.toBase58() == authority!.publicKey.toBase58(),
    );
    assert(merkleTreeAuthority.merkleTreeSetIndex.eq(BN_1));
    assert(merkleTreeAuthority.registeredAssetIndex.eq(BN_0));
    assert(merkleTreeAuthority.enablePermissionlessSplTokens == false);
    assert(
      merkleTreeAuthority.enablePermissionlessMerkleTreeRegistration == false,
    );

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
      this.getMerkleTreeAuthorityPda();
    }
    if (!this.payer) throw new Error("Payer undefined");

    let merkleTreeAuthorityPrior: any = null;
    if (!test) {
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

    if (!test) {
      const merkleTreeAuthority =
        await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
          this.merkleTreeAuthorityPda!,
        );

      assert.equal(
        merkleTreeAuthority.enablePermissionlessSplTokens,
        merkleTreeAuthorityPrior!.enablePermissionlessSplTokens,
      );
      assert.equal(
        merkleTreeAuthority.pubkey.toBase58(),
        newAuthority.toBase58(),
      );
    }
    return txHash;
  }

  async enablePermissionlessSplTokens(configValue: boolean) {
    if (!this.payer) throw new Error("Payer undefined");
    if (this.merkleTreeAuthorityPda == undefined) {
      this.getMerkleTreeAuthorityPda();
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
    const merkleTreeAuthority =
      await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
        this.merkleTreeAuthorityPda!,
      );
    assert(merkleTreeAuthority.enablePermissionlessSplTokens == configValue);
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
      registeredVerifierPda =
        await this.getRegisteredVerifierPda(verifierPubkey);
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
    const registeredVerifierPda = this.registeredVerifierPdas.filter(
      (item: any) => {
        return item.verifierPubkey === verifierPubkey;
      },
    )[0];

    const registeredVerifierAccountInfo =
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
        [Buffer.from(poolType), utf8.encode("pooltype")],
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

    return await sendAndConfirmTransaction(
      this.connection,
      tx,
      [this.payer!],
      confirmConfig,
    );
  }

  async checkPoolRegistered(
    poolPda: any,
    poolType: Array<number>,
    mint: PublicKey | null = null,
  ) {
    if (!this.merkleTreeAuthorityPda)
      throw new Error("merkleTreeAuthorityPda undefined");
    const registeredTokenConfigAccount =
      await this.merkleTreeProgram.account.registeredAssetPool.fetch(
        poolPda.pda,
      );

    const merkleTreeAuthorityPdaAccountInfo =
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

      const registeredTokenAccount = await token.getAccount(
        this.connection,
        poolPda.token,
        "confirmed",
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
          utf8.encode("pool-config"),
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
    const solPoolPda = MerkleTreeConfig.getSolPoolPda(
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
    return PublicKey.findProgramAddressSync(
      [mint.toBytes(), Buffer.from(poolType), utf8.encode("pool")],
      programId,
    )[0];
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
          utf8.encode("pool-config"),
        ],
        this.merkleTreeProgram.programId,
      )[0],
      poolType: poolType,
      token: MerkleTreeConfig.getSplPoolPdaToken(
        mint,
        this.merkleTreeProgram.programId,
        poolType,
      ),
    });
    return this.poolPdas[this.poolPdas.length - 1];
  }

  async getTokenAuthorityPda() {
    this.tokenAuthority = PublicKey.findProgramAddressSync(
      [utf8.encode("spl")],
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

    const splPoolPda = await this.getSplPoolPda(mint, poolType);

    if (!this.tokenAuthority) {
      await this.getTokenAuthorityPda();
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
