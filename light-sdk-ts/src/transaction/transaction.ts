import {
  PublicKey,
  TransactionSignature,
  TransactionInstruction,
  Transaction as SolanaTransaction,
} from "@solana/web3.js";
import { BN, BorshAccountsCoder, Idl, Program, utils } from "@coral-xyz/anchor";
import { N_ASSET_PUBKEYS, Utxo } from "../utxo";
import { PublicInputs, Verifier } from "../verifiers";
import {
  merkleTreeProgramId,
  TransactionErrorCode,
  TransactionError,
  ProviderErrorCode,
  SolMerkleTreeErrorCode,
  Provider,
  sendVersionedTransaction,
  TransactionParameters,
  firstLetterToUpper,
  createAccountObject,
  firstLetterToLower,
} from "../index";
import { IDL_MERKLE_TREE_PROGRAM } from "../idls/index";
import { remainingAccount } from "types/accounts";
const snarkjs = require("snarkjs");
var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, stringifyBigInts, leInt2Buff } = ffjavascript.utils;

// TODO: make dev provide the classification and check here -> it is easier to check whether transaction parameters are plausible

// add verifier class which is passed in with the constructor
// this class replaces the send transaction, also configures path the provingkey and witness, the inputs for the integrity hash
// input custom verifier with three functions by default prepare, proof, send
// include functions from sdk in shieldedTransaction

// TODO: add log option that enables logs

export enum Action {
  SHIELD = "SHIELD",
  TRANSFER = "TRANSFER",
  UNSHIELD = "UNSHIELD",
}

export class Transaction {
  merkleTreeProgram?: Program<typeof IDL_MERKLE_TREE_PROGRAM>;
  shuffleEnabled: Boolean;
  params: TransactionParameters; // contains accounts
  appParams?: any;
  // TODO: relayer shd pls should be part of the provider by default + optional override on Transaction level
  provider: Provider;

  transactionInputs: {
    publicInputs?: PublicInputs;
    rootIndex?: BN;
    proofBytes?: any;
    proofBytesApp?: any;
    publicInputsApp?: PublicInputs;
    encryptedUtxos?: Uint8Array;
  };

  remainingAccounts?: {
    nullifierPdaPubkeys?: remainingAccount[];
    leavesPdaPubkeys?: remainingAccount[];
  };

  proofInput: any;
  proofInputSystem: any;

  /**
   * Initialize transaction
   *
   * @param relayer recipient of the unshielding
   * @param shuffleEnabled
   */
  constructor({
    provider,
    shuffleEnabled = false,
    params,
    appParams,
  }: {
    provider: Provider;
    shuffleEnabled?: boolean;
    params: TransactionParameters;
    appParams?: any;
  }) {
    if (!provider)
      throw new TransactionError(
        TransactionErrorCode.PROVIDER_UNDEFINED,
        "constructor",
      );
    if (!provider.poseidon)
      throw new TransactionError(
        TransactionErrorCode.POSEIDON_HASHER_UNDEFINED,
        "constructor",
        "Poseidon hasher in provider undefined.",
      );
    if (!provider.solMerkleTree)
      throw new TransactionError(
        ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
        "constructor",
        "Merkle tree not set in provider",
      );
    if (!provider.wallet)
      throw new TransactionError(
        TransactionErrorCode.WALLET_UNDEFINED,
        "constructor",
        "Wallet not set in provider.",
      );
    if (!params) {
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "constructor",
      );
    }
    if (!params.verifier)
      throw new TransactionError(
        TransactionErrorCode.VERIFIER_UNDEFINED,
        "constructor",
        "",
      );

    if (params.verifier.config.in.toString() === "4" && !appParams)
      throw new TransactionError(
        TransactionErrorCode.APP_PARAMETERS_UNDEFINED,
        "constructor",
        "For application transactions application parameters need to be specified.",
      );

    if (appParams && params.verifier.config.in.toString() !== "4")
      throw new TransactionError(
        TransactionErrorCode.INVALID_VERIFIER_SELECTED,
        "constructor",
        "For application transactions the verifier needs to be application enabled such as verifier two.",
      );
    this.provider = provider;

    this.shuffleEnabled = shuffleEnabled;
    // TODO: create and check for existence of merkleTreeAssetPubkey depending on utxo asset
    this.params = params;
    this.appParams = appParams;

    //TODO: change to check whether browser/node wallet are the same as signing address
    if (params.action === Action.SHIELD) {
      let wallet = this.provider.wallet;
      if (
        wallet?.publicKey.toBase58() !==
          params.relayer.accounts.relayerPubkey.toBase58() &&
        wallet?.publicKey.toBase58() !==
          params.accounts.signingAddress?.toBase58()
      ) {
        throw new TransactionError(
          TransactionErrorCode.WALLET_RELAYER_INCONSISTENT,
          "constructor",
          `Node or Browser wallet and senderFee used to instantiate yourself as relayer at deposit are inconsistent.`,
        );
      }
    }

    this.transactionInputs = {};
    this.remainingAccounts = {};
  }

  // /** Returns serialized instructions */
  // async proveAndCreateInstructionsJson(): Promise<string[]> {
  //   await this.compileAndProve();
  //   return await this.getInstructionsJson();
  // }

  // async proveAndCreateInstructions(): Promise<TransactionInstruction[]> {
  //   await this.compileAndProve();
  //   if (this.appParams) {
  //     return await this.appParams.verifier.getInstructions(this);
  //   } else if (this.params) {
  //     return await this.params.verifier.getInstructions(this);
  //   } else {
  //     throw new TransactionError(
  //       TransactionErrorCode.NO_PARAMETERS_PROVIDED,
  //       "proveAndCreateInstructions",
  //       "",
  //     );
  //   }
  // }

  async compileAndProve() {
    await this.compile();
    await this.getProof();
    if (this.appParams) {
      await this.getAppProof();
    }
    await this.getRootIndex();
    await this.getPdaAddresses();
  }

  /**
   * @description Prepares proof inputs.
   */
  async compile() {
    this.shuffleUtxos(this.params.inputUtxos);
    this.shuffleUtxos(this.params.outputUtxos);

    if (!this.provider.solMerkleTree)
      throw new TransactionError(
        ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
        "getProofInput",
        "",
      );
    await this.params.getTxIntegrityHash(this.provider.poseidon);
    if (!this.params.txIntegrityHash)
      throw new TransactionError(
        TransactionErrorCode.TX_INTEGRITY_HASH_UNDEFINED,
        "compile",
      );

    const { inputMerklePathIndices, inputMerklePathElements } =
      Transaction.getMerkleProofs(this.provider, this.params.inputUtxos);

    this.proofInputSystem = {
      root: this.provider.solMerkleTree.merkleTree.root(),
      inputNullifier: this.params.inputUtxos.map((x) =>
        x.getNullifier(this.provider.poseidon),
      ),
      // TODO: move public and fee amounts into tx preparation
      publicAmountSpl: this.params.publicAmountSpl.toString(),
      publicAmountSol: this.params.publicAmountSol.toString(),
      publicMintPubkey: this.getMint(),
      inPrivateKey: this.params.inputUtxos?.map((x) => x.account.privkey),
      inPathIndices: inputMerklePathIndices,
      inPathElements: inputMerklePathElements,
      internalTxIntegrityHash: this.params.txIntegrityHash.toString(),
    };
    this.proofInput = {
      transactionVersion: "0",
      txIntegrityHash: this.params.txIntegrityHash.toString(),
      outputCommitment: this.params.outputUtxos.map((x) =>
        x.getCommitment(this.provider.poseidon),
      ),
      inAmount: this.params.inputUtxos?.map((x) => x.amounts),
      inBlinding: this.params.inputUtxos?.map((x) => x.blinding),
      assetPubkeys: this.params.assetPubkeysCircuit,
      outAmount: this.params.outputUtxos?.map((x) => x.amounts),
      outBlinding: this.params.outputUtxos?.map((x) => x.blinding),
      outPubkey: this.params.outputUtxos?.map((x) => x.account.pubkey),
      inIndices: this.getIndices(this.params.inputUtxos),
      outIndices: this.getIndices(this.params.outputUtxos),
      inAppDataHash: this.params.inputUtxos?.map((x) => x.appDataHash),
      outAppDataHash: this.params.outputUtxos?.map((x) => x.appDataHash),
      inPoolType: this.params.inputUtxos?.map((x) => x.poolType),
      outPoolType: this.params.outputUtxos?.map((x) => x.poolType),
      inVerifierPubkey: this.params.inputUtxos?.map(
        (x) => x.verifierAddressCircuit,
      ),
      outVerifierPubkey: this.params.outputUtxos?.map(
        (x) => x.verifierAddressCircuit,
      ),
    };

    if (this.appParams) {
      this.proofInput.transactionHash = Transaction.getTransactionHash(
        this.params,
        this.provider.poseidon,
      );
      this.proofInput.publicAppVerifier = this.appParams.verifier?.pubkey;
    }
  }

  getMint() {
    if (this.params.publicAmountSpl.toString() == "0") {
      return new BN(0);
    } else if (this.params.assetPubkeysCircuit) {
      return this.params.assetPubkeysCircuit[1];
    } else {
      throw new TransactionError(
        TransactionErrorCode.GET_MINT_FAILED,
        "getMint",
        "Get mint failed, transaction parameters dont contain assetPubkeysCircuit but should after normal instantiation",
      );
    }
  }

  async getAppProof() {
    if (!this.appParams)
      throw new TransactionError(
        TransactionErrorCode.APP_PARAMETERS_UNDEFINED,
        "getAppProof",
        "",
      );

    this.appParams.inputs.transactionHash = Transaction.getTransactionHash(
      this.params,
      this.provider.poseidon,
    );

    let { proofBytes, publicInputs } = await this.getProofInternal(
      this.appParams.verifier,
      {
        ...this.appParams.inputs,
        ...this.proofInput,
        inPublicKey: this.params?.inputUtxos?.map(
          (utxo) => utxo.account.pubkey,
        ),
      },
      this.appParams.path,
    );

    this.transactionInputs.proofBytesApp = {
      proofAApp: proofBytes.proofA,
      proofBApp: proofBytes.proofB,
      proofCApp: proofBytes.proofC,
    };
    this.transactionInputs.publicInputsApp = publicInputs;
  }

  async getProof() {
    const path = require("path");
    const firstPath = path.resolve(__dirname, "../../build-circuits/");

    if (!this.params.verifier)
      throw new TransactionError(
        TransactionErrorCode.VERIFIER_UNDEFINED,
        "getProof",
        "",
      );

    let { proofBytes, publicInputs } = await this.getProofInternal(
      this.params?.verifier,
      { ...this.proofInput, ...this.proofInputSystem },
      firstPath,
    );
    this.transactionInputs.proofBytes = proofBytes;
    this.transactionInputs.publicInputs = publicInputs;
  }

  async getProofInternal(verifier: Verifier, inputs: any, firstPath: string) {
    if (!this.proofInput)
      throw new TransactionError(
        TransactionErrorCode.PROOF_INPUT_UNDEFINED,
        "transaction not compiled",
      );

    const completePathWtns = firstPath + "/" + verifier.wtnsGenPath;
    const completePathZkey = firstPath + "/" + verifier.zkeyPath;
    console.time("Proof generation");

    var proof, publicSignals;
    try {
      let res = await snarkjs.groth16.fullProve(
        stringifyBigInts(inputs),
        completePathWtns,
        completePathZkey,
      );
      proof = res.proof;
      publicSignals = res.publicSignals;
    } catch (error) {
      throw new TransactionError(
        TransactionErrorCode.PROOF_GENERATION_FAILED,
        "getProofInternal",
        error,
      );
    }

    console.timeEnd("Proof generation");

    const vKey = await snarkjs.zKey.exportVerificationKey(completePathZkey);
    const res = await snarkjs.groth16.verify(vKey, publicSignals, proof);
    if (res !== true) {
      throw new TransactionError(
        TransactionErrorCode.INVALID_PROOF,
        "getProofInternal",
      );
    }
    const proofJson = JSON.stringify(proof, null, 1);
    const publicInputsJson = JSON.stringify(publicSignals, null, 1);
    try {
      var publicInputsBytesJson = JSON.parse(publicInputsJson.toString());
      var publicInputsBytes = new Array<Array<number>>();
      for (var i in publicInputsBytesJson) {
        let ref: Array<number> = Array.from([
          ...leInt2Buff(unstringifyBigInts(publicInputsBytesJson[i]), 32),
        ]).reverse();
        publicInputsBytes.push(ref);
        // TODO: replace ref, error is that le and be do not seem to be consistent
        // new BN(publicInputsBytesJson[i], "le").toArray("be",32)
        // assert.equal(ref.toString(), publicInputsBytes[publicInputsBytes.length -1].toString());
      }
      const publicInputs =
        verifier.parsePublicInputsFromArray(publicInputsBytes);

      const proofBytes = await Transaction.parseProofToBytesArray(proofJson);
      return { proofBytes, publicInputs };
    } catch (error) {
      console.error("error while generating and validating proof");
      throw error;
    }
  }

  static getTransactionHash(
    params: TransactionParameters,
    poseidon: any,
  ): string {
    if (!params.txIntegrityHash)
      throw new TransactionError(
        TransactionErrorCode.TX_INTEGRITY_HASH_UNDEFINED,
        "getTransactionHash",
      );
    const inputHasher = poseidon.F.toString(
      poseidon(params?.inputUtxos?.map((utxo) => utxo.getCommitment(poseidon))),
    );
    const outputHasher = poseidon.F.toString(
      poseidon(
        params?.outputUtxos?.map((utxo) => utxo.getCommitment(poseidon)),
      ),
    );
    const transactionHash = poseidon.F.toString(
      poseidon([inputHasher, outputHasher, params.txIntegrityHash.toString()]),
    );
    return transactionHash;
  }

  // TODO: add index to merkle tree and check correctness at setup
  // TODO: repeat check for example at tx init to acertain that the merkle tree is not outdated
  /**
   * @description fetches the merkle tree pda from the chain and checks in which index the root of the local merkle tree is.
   */
  async getRootIndex() {
    if (!this.provider.solMerkleTree)
      throw new TransactionError(
        ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
        "getRootIndex",
        "",
      );
    if (!this.provider.solMerkleTree.merkleTree)
      throw new TransactionError(
        SolMerkleTreeErrorCode.MERKLE_TREE_UNDEFINED,
        "getRootIndex",
        "Merkle tree not defined in provider.solMerkleTree",
      );

    if (this.provider.provider && this.provider.solMerkleTree.merkleTree) {
      this.merkleTreeProgram = new Program(
        IDL_MERKLE_TREE_PROGRAM,
        merkleTreeProgramId,
        this.provider.provider,
      );
      let root = Uint8Array.from(
        leInt2Buff(
          unstringifyBigInts(this.provider.solMerkleTree.merkleTree.root()),
          32,
        ),
      );
      let merkle_tree_account_data =
        await this.merkleTreeProgram.account.transactionMerkleTree.fetch(
          this.provider.solMerkleTree.pubkey,
          "confirmed",
        );
      // @ts-ignore: unknown type error
      merkle_tree_account_data.roots.map((x: any, index: any) => {
        if (x.toString() === root.toString()) {
          this.transactionInputs.rootIndex = new BN(index.toString());
        }
      });

      if (this.transactionInputs.rootIndex === undefined) {
        throw new TransactionError(
          TransactionErrorCode.ROOT_NOT_FOUND,
          "getRootIndex",
          `Root index not found for root${root}`,
        );
      }
    } else {
      console.log(
        "provider not defined did not fetch rootIndex set root index to 0",
      );
      this.transactionInputs.rootIndex = new BN(0);
    }
  }

  /**
   * @description Computes the indices in which the asset for the utxo is in the asset pubkeys array.
   * @note Using the indices the zero knowledege proof circuit enforces that only utxos containing the
   * @note assets in the asset pubkeys array are contained in the transaction.
   * @param utxos
   * @returns
   */
  // TODO: make this work for edge case of two 2 different assets plus fee asset in the same transaction
  // TODO: fix edge case of an assetpubkey being 0
  // TODO: !== !! and check non-null
  getIndices(utxos: Utxo[]): string[][][] {
    if (!this.params.assetPubkeysCircuit)
      throw new TransactionError(
        TransactionErrorCode.ASSET_PUBKEYS_UNDEFINED,
        "getIndices",
        "",
      );

    let inIndices: string[][][] = [];
    utxos.map((utxo) => {
      let tmpInIndices: string[][] = [];
      for (var a = 0; a < utxo.assets.length; a++) {
        let tmpInIndices1: string[] = [];

        for (var i = 0; i < N_ASSET_PUBKEYS; i++) {
          try {
            if (
              utxo.assetsCircuit[a].toString() ===
                this.params.assetPubkeysCircuit![i].toString() &&
              !tmpInIndices1.includes("1") &&
              this.params.assetPubkeysCircuit![i].toString() != "0"
            ) {
              tmpInIndices1.push("1");
            } else {
              tmpInIndices1.push("0");
            }
          } catch (error) {
            tmpInIndices1.push("0");
          }
        }

        tmpInIndices.push(tmpInIndices1);
      }

      inIndices.push(tmpInIndices);
    });
    return inIndices;
  }

  /**
   * @description Gets the merkle proofs for every input utxo with amounts > 0.
   * @description For input utxos with amounts == 0 it returns merkle paths with all elements = 0.
   */
  static getMerkleProofs(
    provider: Provider,
    inputUtxos: Utxo[],
  ): {
    inputMerklePathIndices: Array<string>;
    inputMerklePathElements: Array<Array<string>>;
  } {
    if (!provider.solMerkleTree)
      throw new TransactionError(
        SolMerkleTreeErrorCode.MERKLE_TREE_UNDEFINED,
        "getMerkleProofs",
        "",
      );
    if (!provider.solMerkleTree.merkleTree)
      throw new TransactionError(
        SolMerkleTreeErrorCode.MERKLE_TREE_UNDEFINED,
        "getMerkleProofs",
        "",
      );

    var inputMerklePathIndices = new Array<string>();
    var inputMerklePathElements = new Array<Array<string>>();
    // getting merkle proofs
    for (const inputUtxo of inputUtxos) {
      if (
        inputUtxo.amounts[0] > new BN(0) ||
        inputUtxo.amounts[1] > new BN(0)
      ) {
        inputUtxo.index = provider.solMerkleTree.merkleTree.indexOf(
          inputUtxo.getCommitment(provider.poseidon),
        );

        if (inputUtxo.index || inputUtxo.index == 0) {
          if (inputUtxo.index < 0) {
            throw new TransactionError(
              TransactionErrorCode.INPUT_UTXO_NOT_INSERTED_IN_MERKLE_TREE,
              "getMerkleProofs",
              `Input commitment ${inputUtxo.getCommitment(
                provider.poseidon,
              )} was not found`,
            );
          }
          inputMerklePathIndices.push(inputUtxo.index.toString());
          inputMerklePathElements.push(
            provider.solMerkleTree.merkleTree.path(inputUtxo.index)
              .pathElements,
          );
        }
      } else {
        inputMerklePathIndices.push("0");
        inputMerklePathElements.push(
          new Array<string>(provider.solMerkleTree.merkleTree.levels).fill("0"),
        );
      }
    }
    return { inputMerklePathIndices, inputMerklePathElements };
  }

  static getSignerAuthorityPda(
    merkleTreeProgramId: PublicKey,
    verifierProgramId: PublicKey,
  ) {
    return PublicKey.findProgramAddressSync(
      [merkleTreeProgramId.toBytes()],
      verifierProgramId,
    )[0];
  }

  static getRegisteredVerifierPda(
    merkleTreeProgramId: PublicKey,
    verifierProgramId: PublicKey,
  ): PublicKey {
    return PublicKey.findProgramAddressSync(
      [verifierProgramId.toBytes()],
      merkleTreeProgramId,
    )[0];
  }

  // async getInstructionsJson(): Promise<string[]> {
  //   if (!this.params)
  //     throw new TransactionError(
  //       TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
  //       "getInstructionsJson",
  //       "",
  //     );

  //   if (!this.appParams) {
  //     const instructions = await this.params.verifier.getInstructions(this);
  //     let serialized = instructions.map((ix) => JSON.stringify(ix));
  //     return serialized;
  //   } else {
  //     const instructions = await this.appParams.verifier.getInstructions(this);
  //     let serialized = instructions.map((ix: any) => JSON.stringify(ix));
  //     return serialized;
  //   }
  // }

  async sendTransaction(ix: any): Promise<TransactionSignature | undefined> {
    if (this.params.action !== Action.SHIELD) {
      // TODO: replace this with (this.provider.wallet.pubkey != new relayer... this.relayer
      // then we know that an actual relayer was passed in and that it's supposed to be sent to one.
      // we cant do that tho as we'd want to add the default relayer to the provider itself.
      // so just pass in a flag here "shield, unshield, transfer" -> so devs don't have to know that it goes to a relayer.
      // send tx to relayer
      const res = await this.params.relayer.sendTransaction(ix, this.provider);
      return res;
    } else {
      if (!this.provider.provider)
        throw new TransactionError(
          ProviderErrorCode.ANCHOR_PROVIDER_UNDEFINED,
          "sendTransaction",
          "Provider.provider undefined",
        );
      if (!this.params)
        throw new TransactionError(
          TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
          "sendTransaction",
          "",
        );
      if (!this.params.relayer)
        throw new TransactionError(
          TransactionErrorCode.RELAYER_UNDEFINED,
          "sendTransaction",
          "",
        );

      if (this.transactionInputs.rootIndex === undefined) {
        throw new TransactionError(
          TransactionErrorCode.ROOT_INDEX_NOT_FETCHED,
          "sendTransaction",
          "",
        );
      }

      if (!this.remainingAccounts?.leavesPdaPubkeys) {
        throw new TransactionError(
          TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
          "sendTransaction",
          "Run await getPdaAddresses() before invoking sendTransaction",
        );
      }

      const response = await sendVersionedTransaction(ix, this.provider);
      return response;
    }
  }

  async getInstructions(
    params: TransactionParameters,
  ): Promise<TransactionInstruction[]> {
    if (!this.transactionInputs.publicInputs)
      throw new TransactionError(
        TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED,
        "getInstructions",
      );
    if (!params.verifier.verifierProgram)
      throw new TransactionError(
        TransactionErrorCode.VERIFIER_PROGRAM_UNDEFINED,
        "getInstructions",
      );

    const getOrderedInstructionNames = (verifierProgram: Idl) => {
      const orderedInstructionNames = verifierProgram.instructions
        .filter((instruction) =>
          /First|Second|Third|Fourth|Fifth|Sixth|Seventh|Eighth|Ninth/.test(
            instruction.name,
          ),
        )
        .sort((a, b) => {
          const suffixes = [
            "First",
            "Second",
            "Third",
            "Fourth",
            "Fifth",
            "Sixth",
            "Seventh",
            "Eighth",
            "Ninth",
          ];
          const aIndex = suffixes.findIndex((suffix) =>
            a.name.endsWith(suffix),
          );
          const bIndex = suffixes.findIndex((suffix) =>
            b.name.endsWith(suffix),
          );

          if (aIndex === 7 || bIndex === 7) {
            throw new Error("Found an instruction with the 'Eighth' suffix.");
          }

          return aIndex - bIndex;
        })
        .map((instruction) => instruction.name);

      return orderedInstructionNames;
    };
    let inputObject = {
      ...this.transactionInputs.proofBytes,
      ...this.transactionInputs.proofBytesApp,
      ...this.transactionInputs.publicInputsApp,
      ...this.transactionInputs.publicInputs,
      rootIndex: this.transactionInputs.rootIndex,
      relayerFee: this.params.relayer.getRelayerFee(this.params.ataCreationFee),
      encryptedUtxos: Buffer.from(this.params.encryptedUtxos!),
    };

    var instructions = [];
    const instructionNames = getOrderedInstructionNames(params.verifier.idl);
    for (let i = 0; i < instructionNames.length; i++) {
      const instruction = instructionNames[i];
      const coder = new BorshAccountsCoder(params.verifier.idl);

      const accountName = "instructionData" + firstLetterToUpper(instruction);
      let inputs = createAccountObject(
        inputObject,
        params.verifier.idl.accounts!,
        accountName,
      );

      let inputsVec = await coder.encode(accountName, inputs);

      const methodName = firstLetterToLower(instruction);

      const method = params.verifier.verifierProgram.methods[
        methodName as keyof typeof params.verifier.verifierProgram.methods
      ](inputsVec).accounts({
        ...this.params.accounts,
        ...this.params.relayer.accounts,
        verifierProgram: this.params.verifier.verifierProgram?.programId,
      });

      // Check if it's the last iteration
      if (i === instructionNames.length - 1) {
        method.remainingAccounts([
          ...this.remainingAccounts!.nullifierPdaPubkeys!,
          ...this.remainingAccounts!.leavesPdaPubkeys!,
        ]);
      }

      const ix = await method.instruction();

      instructions?.push(ix);
    }
    return instructions;
  }

  async sendAndConfirmTransaction(): Promise<TransactionSignature> {
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "sendAndConfirmTransaction",
        "",
      );

    if (!this.provider.wallet)
      throw new TransactionError(
        TransactionErrorCode.WALLET_UNDEFINED,
        "sendAndConfirmTransaction",
        "Cannot use sendAndConfirmTransaction without wallet",
      );

    await this.getRootIndex();
    await this.getPdaAddresses();

    var instructions;

    if (!this.appParams) {
      instructions = await this.getInstructions(this.params);
    } else {
      instructions = await this.getInstructions(this.appParams);
    }

    if (instructions) {
      let tx = "Something went wrong";
      for (var ix in instructions) {
        let txTmp = await this.sendTransaction(instructions[ix]);
        if (txTmp) {
          console.log("tx : ", txTmp);
          await this.provider.provider?.connection.confirmTransaction(
            txTmp,
            "confirmed",
          );
          tx = txTmp;
        } else {
          throw new TransactionError(
            TransactionErrorCode.SEND_TRANSACTION_FAILED,
            "sendAndConfirmTransaction",
            "",
          );
        }
      }
      return tx;
    } else {
      throw new TransactionError(
        TransactionErrorCode.GET_INSTRUCTIONS_FAILED,
        "sendAndConfirmTransaction",
        "",
      );
    }
  }

  // TODO: deal with this: set own payer just for that? where is this used?
  // This is used by applications not the relayer
  async closeVerifierState(): Promise<TransactionSignature> {
    if (!this.provider.wallet)
      throw new TransactionError(
        TransactionErrorCode.WALLET_UNDEFINED,
        "closeVerifierState",
        "Cannot use closeVerifierState without wallet",
      );
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "closeVerifierState",
        "",
      );
    if (!this.params.verifier.verifierProgram)
      throw new TransactionError(
        TransactionErrorCode.VERIFIER_PROGRAM_UNDEFINED,
        "closeVerifierState",
        "",
      );
    if (this.appParams) {
      const transaction = new SolanaTransaction().add(
        await this.appParams?.verifier.verifierProgram.methods
          .closeVerifierState()
          .accounts({
            ...this.params.accounts,
          })
          .instruction(),
      );

      return await this.provider.wallet!.sendAndConfirmTransaction(transaction);
    } else {
      const transaction = new SolanaTransaction().add(
        await this.params?.verifier.verifierProgram.methods
          .closeVerifierState()
          .accounts({
            ...this.params.accounts,
          })
          .instruction(),
      );

      return await this.provider.wallet!.sendAndConfirmTransaction(transaction);
    }
  }

  getPdaAddresses() {
    if (!this.transactionInputs.publicInputs)
      throw new TransactionError(
        TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED,
        "getPdaAddresses",
        "",
      );
    if (!this.params.verifier.verifierProgram)
      throw new TransactionError(
        TransactionErrorCode.VERIFIER_PROGRAM_UNDEFINED,
        "getPdaAddresses",
        "",
      );
    if (!this.params.relayer)
      throw new TransactionError(
        TransactionErrorCode.RELAYER_UNDEFINED,
        "getPdaAddresses",
        "",
      );
    if (!this.remainingAccounts)
      throw new TransactionError(
        TransactionErrorCode.REMAINING_ACCOUNTS_NOT_CREATED,
        "getPdaAddresses",
        "Remaining accounts undefined",
      );

    let nullifiers = this.transactionInputs.publicInputs.nullifiers;
    let signer = this.params.relayer.accounts.relayerPubkey;

    this.remainingAccounts.nullifierPdaPubkeys = [];
    for (var i in nullifiers) {
      this.remainingAccounts.nullifierPdaPubkeys.push({
        isSigner: false,
        isWritable: true,
        pubkey: PublicKey.findProgramAddressSync(
          [Uint8Array.from([...nullifiers[i]]), utils.bytes.utf8.encode("nf")],
          merkleTreeProgramId,
        )[0],
      });
    }

    this.remainingAccounts.leavesPdaPubkeys = [];
    for (
      var j = 0;
      j < this.transactionInputs.publicInputs.leaves.length;
      j += 2
    ) {
      this.remainingAccounts.leavesPdaPubkeys.push({
        isSigner: false,
        isWritable: true,
        pubkey: PublicKey.findProgramAddressSync(
          [
            Buffer.from(
              Array.from(
                this.transactionInputs.publicInputs.leaves[j],
              ).reverse(),
            ),
            utils.bytes.utf8.encode("leaves"),
          ],
          merkleTreeProgramId,
        )[0],
      });
    }

    if (this.appParams) {
      this.params.accounts.verifierState = PublicKey.findProgramAddressSync(
        [signer.toBytes(), utils.bytes.utf8.encode("VERIFIER_STATE")],
        this.appParams.verifier.verifierProgram.programId,
      )[0];
    } else {
      this.params.accounts.verifierState = PublicKey.findProgramAddressSync(
        [signer.toBytes(), utils.bytes.utf8.encode("VERIFIER_STATE")],
        this.params.verifier.verifierProgram.programId,
      )[0];
    }
  }

  // TODO: use higher entropy rnds
  shuffleUtxos(utxos: Utxo[]) {
    if (this.shuffleEnabled) {
      console.log("shuffling utxos");
    } else {
      console.log("shuffle disabled");
      return;
    }
    let currentIndex: number = utxos.length;
    let randomIndex: number;

    // While there remain elements to shuffle...
    while (0 !== currentIndex) {
      // Pick a remaining element...
      randomIndex = Math.floor(Math.random() * currentIndex);
      currentIndex--;

      // And swap it with the current element.
      [utxos[currentIndex], utxos[randomIndex]] = [
        utxos[randomIndex],
        utxos[currentIndex],
      ];
    }

    return utxos;
  }

  // also converts lE to BE
  static async parseProofToBytesArray(data: any) {
    var mydata = JSON.parse(data.toString());

    for (var i in mydata) {
      if (i == "pi_a" || i == "pi_c") {
        for (var j in mydata[i]) {
          mydata[i][j] = Array.from(
            leInt2Buff(unstringifyBigInts(mydata[i][j]), 32),
          ).reverse();
        }
      } else if (i == "pi_b") {
        for (var j in mydata[i]) {
          for (var z in mydata[i][j]) {
            mydata[i][j][z] = Array.from(
              leInt2Buff(unstringifyBigInts(mydata[i][j][z]), 32),
            );
          }
        }
      }
    }
    return {
      proofA: [mydata.pi_a[0], mydata.pi_a[1]].flat(),
      proofB: [
        mydata.pi_b[0].flat().reverse(),
        mydata.pi_b[1].flat().reverse(),
      ].flat(),
      proofC: [mydata.pi_c[0], mydata.pi_c[1]].flat(),
    };
  }

  static getTokenAuthority(): PublicKey {
    return PublicKey.findProgramAddressSync(
      [utils.bytes.utf8.encode("spl")],
      merkleTreeProgramId,
    )[0];
  }
}
