import * as anchor from "@project-serum/anchor";
const solana = require("@solana/web3.js");
import { MerkleTreeProgram } from "../../target/types/merkle_tree_program";

import {
  DEFAULT_PROGRAMS
} from "./constants";

export class MerkleTreeConfig {
  constructor({
    merkleTreePubkey,
    payer,
  }) {
      this.merkleTreePubkey =merkleTreePubkey;
      this.payer = payer;
      this.merkleTreeProgram = anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;
      // TODO: reorg pool pdas, have one object per pool type and then an array with registered pools of this type
      this.poolPdas = [];
      this.poolTypes = [];
      this.registeredVerifierPdas = [];
    }
    async getMerkleTreeAccounts() {
      this.MERKLE_TREE_AUTHORITY_PDA = (await solana.PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY")],
        this.merkleTreeProgram.programId
      ))[0];

      this.MERKLE_TREE_KEY = (await solana.PublicKey.findProgramAddress(
          [merkleTreeProgram.programId.toBuffer()], // , Buffer.from(new Uint8Array(8).fill(0))
          merkleTreeProgram.programId))[0];

      this.PRE_INSERTED_LEAVES_INDEX = (await solana.PublicKey.findProgramAddress(
          [MERKLE_TREE_KEY.toBuffer()],
          merkleTreeProgram.programId))[0];
      this.TOKEN_AUTHORITY = (await solana.PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("spl")],
        merkleTreeProgram.programId
      ))[0];
    }

    async getMerkleTreeAuthorityPda() {
      this.merkleTreeAuthorityPda = (await solana.PublicKey.findProgramAddress(
          [anchor.utils.bytes.utf8.encode("MERKLE_TREE_AUTHORITY")],
          this.merkleTreeProgram.programId
        ))[0];
      return this.merkleTreeAuthorityPda;
    }

    async initMerkleTreeAuthority(authority) {
      if (authority == undefined) {
        authority = this.payer
      }
      if (this.merkleTreeAuthorityPda == undefined) {
        await this.getMerkleTreeAuthorityPda()
      }

      const tx = await this.merkleTreeProgram.methods.initializeMerkleTreeAuthority().accounts({
        authority: authority.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([authority])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});

      return tx;
    }

    async updateMerkleTreeAuthority(newAuthority) {
      if (this.merkleTreeAuthorityPda == undefined) {
        await this.getMerkleTreeAuthorityPda()
      }
      const tx = await this.merkleTreeProgram.methods.updateMerkleTreeAuthority().accounts({
        authority: this.payer.publicKey,
        newAuthority,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});

      return tx;
    }

    async enableNfts(configValue) {
      if (this.merkleTreeAuthorityPda == undefined) {
        await this.getMerkleTreeAuthorityPda()
      }
      const tx = await this.merkleTreeProgram.methods.enableNfts(configValue).accounts({
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});
      return tx;
    }

    async enablePermissionlessSplTokens(configValue) {
      if (this.merkleTreeAuthorityPda == undefined) {
        await this.getMerkleTreeAuthorityPda()
      }
      const tx = await this.merkleTreeProgram.methods.enablePermissionlessSplTokens(configValue).accounts({
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "finalized", preflightCommitment: 'finalized',});
      return tx;
    }

    async getRegisteredVerifierPda(verifierPubkey) {
      // TODO: add check whether already exists
      this.registeredVerifierPdas.push({registeredVerifierPda: (await solana.PublicKey.findProgramAddress(
          [verifierPubkey.toBuffer()],
          this.merkleTreeProgram.programId
        ))[0],
        verifierPubkey: verifierPubkey
      });
      return this.registeredVerifierPdas[this.registeredVerifierPdas.length - 1];
    }

    async registerVerifier(verifierPubkey) {
      let registeredVerifierPda = this.registeredVerifierPdas.filter((item) => {return item.verifierPubkey === verifierPubkey})[0]

      if (!registeredVerifierPda) {
          registeredVerifierPda = await this.getRegisteredVerifierPda(verifierPubkey)
      }

      const tx = await this.merkleTreeProgram.methods.registerVerifier(
        verifierPubkey
      ).accounts({
        registeredVerifierPda: registeredVerifierPda.registeredVerifierPda,
        authority: this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "confirmed", preflightCommitment: 'confirmed',});

      return tx;
    }

    async getPoolTypePda(poolType) {
      if (poolType.length != 32) {
        throw `invalid pooltype length ${poolType.length}`
      }
      // TODO: add check whether already exists

      this.poolTypes.push({poolPda: (await solana.PublicKey.findProgramAddress(
          [poolType, anchor.utils.bytes.utf8.encode("pooltype")],
          this.merkleTreeProgram.programId
        ))[0], poolType: poolType});
        return this.poolTypes[this.poolTypes.length - 1];
    }

    async registerPoolType(poolType) {
      let registeredPoolTypePda = this.poolTypes.filter((item) => {return item.poolType === poolType})[0]

      if (!registeredPoolTypePda) {
          registeredPoolTypePda = await this.getPoolTypePda(poolType)
      }
      console.log(registeredPoolTypePda);

      const tx = await this.merkleTreeProgram.methods.registerPoolType(
        Buffer.from(new Uint8Array(32).fill(0))
      ).accounts({
        registeredPoolTypePda:  registeredPoolTypePda.poolPda,
        authority:              this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "confirmed", preflightCommitment: 'confirmed',});
      return tx;
    }


    async getSolPoolPda(poolType) {
      this.poolPdas.push({pda: (await solana.PublicKey.findProgramAddress(
        [new Uint8Array(32).fill(0), new Uint8Array(32).fill(0), anchor.utils.bytes.utf8.encode("pool-config")],
          this.merkleTreeProgram.programId
        ))[0], poolType: poolType});
        return this.poolPdas[this.poolPdas.length - 1]
    }

    async registerSolPool(poolType) {
      let registeredPoolTypePda = this.poolTypes.filter((item) => {return item.poolType === poolType})[0]

      if (!registeredPoolTypePda) {
          registeredPoolTypePda = await this.getPoolTypePda(poolType)
      }
      let solPoolPda = await this.getSolPoolPda(poolType);


      const tx = await this.merkleTreeProgram.methods.registerSolPool(
      ).accounts({
        registeredAssetPoolPda:  solPoolPda.pda,
        authority:              this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        registeredPoolTypePda:  registeredPoolTypePda.poolPda,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "confirmed", preflightCommitment: 'confirmed',});

      return tx;
    }

    async getSplPoolPdaToken(poolType, mint) {
      let pda = (await solana.PublicKey.findProgramAddress(
        [mint.toBytes(), new Uint8Array(32).fill(0), anchor.utils.bytes.utf8.encode("pool")],
          this.merkleTreeProgram.programId
        ))[0];
        return pda;
    }

    async getSplPoolPda(poolType, mint) {
      this.poolPdas.push({pda: (await solana.PublicKey.findProgramAddress(
        [mint.toBytes(), new Uint8Array(32).fill(0), anchor.utils.bytes.utf8.encode("pool-config")],
          this.merkleTreeProgram.programId
        ))[0], poolType: poolType, token: await this.getSplPoolPdaToken(poolType, mint)});
      return this.poolPdas[this.poolPdas.length - 1];
    }

    async getTokeAuthority() {
      this.tokenAuthority = (await solana.PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("spl")],
        this.merkleTreeProgram.programId
      ))[0];
    }

    async registerSplPool(poolType, mint) {
      let registeredPoolTypePda = this.poolTypes.filter((item) => {return item.poolType === poolType})[0]

      if (!registeredPoolTypePda) {
          registeredPoolTypePda = await this.getPoolTypePda(poolType)
      }
      console.log(" mint ", mint);

      let splPoolPda = await this.getSplPoolPda(poolType, mint);

      if (!this.tokenAuthority) {
          await this.getTokeAuthority()
      }

      console.log("splPoolPda.pda ", splPoolPda.pda.toBase58());

      const tx = await this.merkleTreeProgram.methods.registerSplPool(
      ).accounts({
        registeredAssetPoolPda:  splPoolPda.pda,
        authority:              this.payer.publicKey,
        merkleTreeAuthorityPda: this.merkleTreeAuthorityPda,
        registeredPoolTypePda:  registeredPoolTypePda.poolPda,
        merkleTreePdaToken: splPoolPda.token,
        tokenAuthority: this.tokenAuthority,
        mint,
        ...DEFAULT_PROGRAMS
      })
      .signers([this.payer])
      .rpc({commitment: "confirmed", preflightCommitment: 'confirmed',});

      return tx;
    }
}
