// @ts-nocheck
import * as anchor from "@coral-xyz/anchor";
import {
  MerkleTreeProgram,
  MerkleTreeProgramIdl,
} from "../idls/merkle_tree_program";
import { assert, expect } from "chai";
const token = require("@solana/spl-token");
import { Connection, PublicKey, Keypair } from "@solana/web3.js";

import { confirmConfig, DEFAULT_PROGRAMS, merkleTreeProgramId } from "../index";
import { Program } from "@coral-xyz/anchor";

export class MerkleTreeConfig {
  merkleTreeProgram: Program<MerkleTreeProgramIdl>;
  merkleTreePubkey: PublicKey;
  connection: Connection;
  registeredVerifierPdas: any;
  preInsertedLeavesIndex?: PublicKey;
  merkleTreeAuthorityPda?: PublicKey;
  payer: Keypair;
  tokenAuthority?: PublicKey;
  provider: anchor.Provider;

  constructor({
    merkleTreePubkey,
    payer,
    connection,
    provider,
  }: {
    merkleTreePubkey: PublicKey;
    payer?: Keypair;
    connection: Connection;
    provider: anchor.Provider;
  }) {
    this.merkleTreePubkey = merkleTreePubkey;
    this.payer = payer;
    this.provider = provider;
    this.merkleTreeProgram = new Program(
      MerkleTreeProgram,
      merkleTreeProgramId,
      provider,
    );
    // TODO: reorg pool pdas, have one object per pool type and then an array with registered pools of this type
    this.poolPdas = [];
    this.poolTypes = [];
    this.registeredVerifierPdas = [];
    this.connection = connection;
  }

  async getPreInsertedLeavesIndex() {
    this.preInsertedLeavesIndex = PublicKey.findProgramAddressSync(
      [this.merkleTreePubkey.toBuffer()],
      this.merkleTreeProgram.programId,
    )[0];
    return this.preInsertedLeavesIndex;
  }

  async initializeNewMerkleTree(merkleTreePubkey?: PublicKey) {
    if (merkleTreePubkey) {
      this.merkleTreePubkey = merkleTreePubkey;
    }
    await this.getPreInsertedLeavesIndex();
    await this.getMerkleTreeAuthorityPda();
    const tx = await this.merkleTreeProgram.methods
      .initializeNewMerkleTree(new anchor.BN("50"))
      .accounts({
        authority: this.payer.publicKey,
        merkleTree: this.merkleTreePubkey,
        preInsertedLeavesIndex: this.preInsertedLeavesIndex,
        systemProgram: DEFAULT_PROGRAMS.systemProgram,
        rent: DEFAULT_PROGRAMS.rent,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
      })
      .signers([this.payer])
      .rpc(confirmConfig);

    await this.checkMerkleTreeIsInitialized();
    await this.checkPreInsertedLeavesIndexIsInitialized();
    return tx;
  }

  async checkMerkleTreeIsInitialized() {
    var merkleTreeAccountInfo =
      await this.merkleTreeProgram.account.merkleTree.fetch(
        this.merkleTreePubkey,
        confirmConfig,
      );
    assert(
      merkleTreeAccountInfo != null,
      "merkleTreeAccountInfo not initialized",
    );
    // zero values
    // index == 0
    // roots are empty save for 0
    // lock duration is correct
    assert(merkleTreeAccountInfo.lockDuration.toString() == "50");
  }

  async checkPreInsertedLeavesIndexIsInitialized() {
    var preInsertedLeavesIndexAccountInfo =
      await this.merkleTreeProgram.account.preInsertedLeavesIndex.fetch(
        this.preInsertedLeavesIndex,
        confirmConfig,
      );

    assert(
      preInsertedLeavesIndexAccountInfo != null,
      "preInsertedLeavesIndexAccountInfo not initialized",
    );
    assert(preInsertedLeavesIndexAccountInfo.nextIndex.toString() == "0");
  }

  async printMerkleTree() {
    var merkleTreeAccountInfo =
      await this.merkleTreeProgram.account.merkleTree.fetch(
        this.merkleTreePubkey,
      );
    console.log("merkleTreeAccountInfo ", merkleTreeAccountInfo);
  }

  async getMerkleTreeAuthorityPda() {
    this.merkleTreeAuthorityPda = PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY")],
      this.merkleTreeProgram.programId,
    )[0];
    return this.merkleTreeAuthorityPda;
  }

  async initMerkleTreeAuthority(authority?: Keypair | undefined) {
    try {
      if (authority == undefined) {
        authority = this.payer;
      }

      if (this.merkleTreeAuthorityPda == undefined) {
        await this.getMerkleTreeAuthorityPda();
      }

      const tx = await this.merkleTreeProgram.methods
        .initializeMerkleTreeAuthority()
        .accounts({
          authority: authority.publicKey,
          merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
          ...DEFAULT_PROGRAMS,
        })
        .signers([authority])
        .rpc(confirmConfig);

      // await sendAndConfirmTransaction(this.connection, new Transaction([authority]).add(tx), [authority], confirmConfig);
      // rpc(confirmConfig);
      assert(
        this.connection.getAccountInfo(
          this.merkleTreeAuthorityPda,
          "confirmed",
        ) != null,
        "init authority failed",
      );

      let merkleTreeAuthority =
        await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
          this.merkleTreeAuthorityPda,
          confirmConfig,
        );

      assert(merkleTreeAuthority.enablePermissionlessSplTokens == false);
      assert(merkleTreeAuthority.enableNfts == false);
      assert(
        merkleTreeAuthority.pubkey.toBase58() == authority.publicKey.toBase58(),
      );
      assert(merkleTreeAuthority.registeredAssetIndex.toString() == "0");

      return tx;
    } catch (err) {
      throw err;
    }
  }

  async updateMerkleTreeAuthority(newAuthority: PublicKey, test = false) {
    if (!this.merkleTreeAuthorityPda) {
      await this.getMerkleTreeAuthorityPda();
    }
    let merkleTreeAuthorityPrior;
    if (test != true) {
      merkleTreeAuthorityPrior =
        await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
          this.merkleTreeAuthorityPda,
        );
      if (merkleTreeAuthorityPrior == null) {
        throw `Merkle tree authority ${this.merkleTreeAuthorityPda.toBase58()} not initialized`;
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
      .rpc(confirmConfig);

    if (test != true) {
      let merkleTreeAuthority =
        await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
          this.merkleTreeAuthorityPda,
        );

      assert(
        merkleTreeAuthority.enablePermissionlessSplTokens ==
          merkleTreeAuthorityPrior.enablePermissionlessSplTokens,
      );
      assert(
        merkleTreeAuthority.enableNfts == merkleTreeAuthorityPrior.enableNfts,
      );
      assert(merkleTreeAuthority.pubkey.toBase58() == newAuthority.toBase58());
    }
    return tx;
  }

  async enableNfts(configValue: Boolean) {
    if (this.merkleTreeAuthorityPda == undefined) {
      await this.getMerkleTreeAuthorityPda();
    }
    const tx = await this.merkleTreeProgram.methods
      .enableNfts(configValue)
      .accounts({
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS,
      })
      .signers([this.payer])
      .rpc(confirmConfig);
    let merkleTreeAuthority =
      await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
        this.merkleTreeAuthorityPda,
      );
    assert(merkleTreeAuthority.enableNfts == configValue);

    return tx;
  }

  async enablePermissionlessSplTokens(configValue: Boolean) {
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
      .rpc(confirmConfig);
    let merkleTreeAuthority =
      await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
        this.merkleTreeAuthorityPda,
      );
    assert(merkleTreeAuthority.enablePermissionlessSplTokens == configValue);
    return tx;
  }

  async updateLockDuration(lockDuration: Number) {
    if (this.merkleTreeAuthorityPda == undefined) {
      await this.getMerkleTreeAuthorityPda();
    }

    const tx = await this.merkleTreeProgram.methods
      .updateLockDuration(new anchor.BN(lockDuration.toString()))
      .accounts({
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        merkleTree: this.merkleTreePubkey,
        ...DEFAULT_PROGRAMS,
      })
      .signers([this.payer])
      .rpc(confirmConfig);
    let merkleTree = await this.merkleTreeProgram.account.merkleTree.fetch(
      this.merkleTreePubkey,
    );
    assert(merkleTree.lockDuration == lockDuration);
    console.log("lock duration updated to: ", lockDuration);

    return tx;
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
    let registeredVerifierPda = this.registeredVerifierPdas.filter((item) => {
      return item.verifierPubkey === verifierPubkey;
    })[0];

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
      .rpc(confirmConfig);

    await this.checkVerifierIsRegistered(verifierPubkey);

    return tx;
  }

  async checkVerifierIsRegistered(verifierPubkey: PublicKey) {
    let registeredVerifierPda = this.registeredVerifierPdas.filter((item) => {
      return item.verifierPubkey === verifierPubkey;
    })[0];

    var registeredVerifierAccountInfo =
      await this.merkleTreeProgram.account.registeredVerifier.fetch(
        registeredVerifierPda.registeredVerifierPda,
        confirmConfig,
      );

    assert(registeredVerifierAccountInfo != null);
    assert(
      registeredVerifierAccountInfo.pubkey.toBase58() ==
        verifierPubkey.toBase58(),
    );
  }

  async getPoolTypePda(poolType) {
    if (poolType.length != 32) {
      throw `invalid pooltype length ${poolType.length}`;
    }
    // TODO: add check whether already exists

    this.poolTypes.push({
      poolPda: PublicKey.findProgramAddressSync(
        [poolType, anchor.utils.bytes.utf8.encode("pooltype")],
        this.merkleTreeProgram.programId,
      )[0],
      poolType: poolType,
    });
    return this.poolTypes[this.poolTypes.length - 1];
  }

  async registerPoolType(poolType) {
    let registeredPoolTypePda = this.poolTypes.filter((item) => {
      return item.poolType === poolType;
    })[0];

    if (!registeredPoolTypePda) {
      registeredPoolTypePda = await this.getPoolTypePda(poolType);
    }

    const tx = await this.merkleTreeProgram.methods
      .registerPoolType(Buffer.from(new Uint8Array(32).fill(0)))
      .accounts({
        registeredPoolTypePda: registeredPoolTypePda.poolPda,
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS,
      })
      .signers([this.payer])
      .rpc(confirmConfig);
    return tx;
  }

  async checkPoolRegistered(poolPda, poolType, mint: PublicKey | null = null) {
    var registeredTokenConfigAccount =
      await this.merkleTreeProgram.account.registeredAssetPool.fetch(
        poolPda.pda,
      );

    var merkleTreeAuthorityPdaAccountInfo =
      await this.merkleTreeProgram.account.merkleTreeAuthority.fetch(
        this.merkleTreeAuthorityPda,
      );
    assert(
      registeredTokenConfigAccount.poolType.toString() == poolType.toString(),
    );
    assert(
      registeredTokenConfigAccount.index.toString() ==
        (merkleTreeAuthorityPdaAccountInfo.registeredAssetIndex - 1).toString(),
    );

    if (mint !== null) {
      assert(
        registeredTokenConfigAccount.assetPoolPubkey.toBase58() ==
          poolPda.token.toBase58(),
      );

      var registeredTokenAccount = await token.getAccount(
        this.connection,
        poolPda.token,
        { commitment: "confirmed", preflightCommitment: "confirmed" },
      );
      assert(registeredTokenAccount != null);
      assert(registeredTokenAccount.mint.toBase58() == mint.toBase58());
    } else {
      assert(
        registeredTokenConfigAccount.assetPoolPubkey.toBase58() ==
          poolPda.pda.toBase58(),
      );
    }
  }

  static getSolPoolPda(
    programId: PublicKey,
    poolType: Uint8Array = new Uint8Array(32),
  ) {
    return {
      pda: PublicKey.findProgramAddressSync(
        [
          new Uint8Array(32).fill(0),
          poolType,
          anchor.utils.bytes.utf8.encode("pool-config"),
        ],
        programId,
      )[0],
      poolType: poolType,
    };
  }

  async registerSolPool(poolType: Uint8Array) {
    let registeredPoolTypePda = this.poolTypes.filter((item) => {
      return item.poolType === poolType;
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
      .rpc({ commitment: "confirmed", preflightCommitment: "confirmed" });

    await this.checkPoolRegistered(solPoolPda, poolType);
    console.log("registered sol pool ", this.merkleTreeAuthorityPda.toBase58());

    return tx;
  }

  static getSplPoolPdaToken(
    mint: PublicKey,
    programId: PublicKey,
    poolType: Uint8Array = new Uint8Array(32).fill(0),
  ) {
    let pda = PublicKey.findProgramAddressSync(
      [mint.toBytes(), poolType, anchor.utils.bytes.utf8.encode("pool")],
      programId,
    )[0];
    return pda;
  }

  async getSplPoolPda(
    mint: PublicKey,
    poolType: Uint8Array = new Uint8Array(32).fill(0),
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

  async registerSplPool(poolType: Uint8Array, mint: PublicKey) {
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
      .rpc(confirmConfig);

    await this.checkPoolRegistered(splPoolPda, poolType, mint);

    return tx;
  }
}
