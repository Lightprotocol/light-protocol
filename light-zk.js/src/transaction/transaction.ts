import {
  PublicKey,
  TransactionSignature,
  TransactionInstruction,
  Transaction as SolanaTransaction,
} from "@solana/web3.js";
import { BN, BorshAccountsCoder, Idl, Program, utils } from "@coral-xyz/anchor";
import { N_ASSET_PUBKEYS, Utxo } from "../utxo";
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
  hashAndTruncateToCircuit,
} from "../index";
import { IDL_MERKLE_TREE_PROGRAM } from "../idls/index";
import { remainingAccount } from "../types/accounts";
import { Prover } from "./prover";

var ffjavascript = require("ffjavascript");
const { unstringifyBigInts, leInt2Buff } = ffjavascript.utils;

const path = require("path");

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

type PublicInputs = {
  root: Array<number>;
  publicAmountSpl: Array<number>;
  txIntegrityHash: Array<number>;
  publicAmountSol: Array<number>;
  publicMintPubkey: Array<number>;
  inputNullifier: Array<Array<number>>;
  outputCommitment: Array<Array<number>>;
  // only for app verifiers
  transactionHash?: Array<number>;
  checkedParams?: Array<Array<number>>;
  publicAppVerifier?: Array<number>;
};

/**
 * The Transaction class represents a transaction in the context of the application.
 * - It handles various transaction operations like fetching PDA addresses, shuffling UTXOs, sending transactions and more.
 * - This class encapsulates data related to a transaction, including inputs, parameters, remaining accounts and more.
 *
 * @property {Boolean} shuffleEnabled - A flag to enable shuffling of UTXOs.
 * @property {TransactionParameters} params - Contains all the parameters required for a transaction.
 * @property {any} appParams - Parameters required for application-specific transactions.
 * @property {Provider} provider - The provider used for the transaction.
 * @property {Object} transactionInputs - The inputs for the transaction.
 * @property {Object} remainingAccounts - The remaining accounts after the transaction.
 */
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
    publicInputsApp?: any;
    encryptedUtxos?: Uint8Array;
  };

  remainingAccounts?: {
    nullifierPdaPubkeys?: remainingAccount[];
    leavesPdaPubkeys?: remainingAccount[];
  };

  proofInput: any;
  firstPath!: string;

  /**
   * Creates an instance of the Transaction class.
   *
   * @param {Object} params - The parameters for the constructor.
   * @param {Provider} params.provider - The provider used for the transaction.
   * @param {boolean} params.shuffleEnabled - A flag to enable shuffling of UTXOs.
   * @param {TransactionParameters} params.params - Contains all the parameters required for a transaction.
   * @param {any} params.appParams - Parameters required for application-specific transactions.
   *
   * @throws {TransactionError} TransactionError:
   * - When the verifier needs to be application enabled but it's not.
   * - When the node or browser wallet and senderFee used to instantiate yourself as relayer at deposit are inconsistent.
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
    if (!params.verifierIdl)
      throw new TransactionError(
        TransactionErrorCode.VERIFIER_IDL_UNDEFINED,
        "constructor",
        "",
      );

    if (params.verifierConfig.in.toString() === "4" && !appParams)
      throw new TransactionError(
        TransactionErrorCode.APP_PARAMETERS_UNDEFINED,
        "constructor",
        "For application transactions application parameters need to be specified.",
      );

    if (appParams && params.verifierConfig.in.toString() !== "4")
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

  // TODO: evaluate whether we need this function
  // /** Returns serialized instructions */
  // async proveAndCreateInstructionsJson(): Promise<string[]> {
  //   await this.compileAndProve();
  //   return await this.getInstructionsJson();
  // }

  // TODO: evaluate whether we need this function
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

  /**
   * @async
   * This asynchronous method is an extension of the compile method that includes additional steps.
   *  - It not only compiles the transaction which includes preparing proof inputs and arranging
   * the UTXOs in a specific order, but also generates the zk-SNARK proofs, fetches the root index from the on-chain
   * Merkle tree and calculates the Program Derived Addresses (PDAs).
   *
   * - Next, it generates the proof for the transaction. If application parameters (`appParams`) are
   * defined, it generates an additional proof for application-specific logic.
   *
   * - The method then fetches the index of the root of the local Merkle tree in the Merkle tree stored
   * on-chain. This is an important step in validating the transaction, as it verifies that the state
   * of the transaction matches the state of the on-chain Merkle tree.
   *
   * - Finally, it calculates and stores the Program Derived Addresses (PDAs) for the transaction. PDAs
   * are used in the Solana program to handle accounts that are dynamically created during the execution
   * of the program.
   *
   * @returns {Promise<void>} Returns a Promise that resolves when the method has finished executing.
   *
   */
  async compileAndProve() {
    await this.compile();
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.TX_PARAMETERS_UNDEFINED,
        "compileAndProve",
      );
    await this.getProof();
    if (this.appParams) {
      await this.getAppProof();
    }
    await this.getRootIndex();
    this.getPdaAddresses();
  }

  /**
   * @async
   * This asynchronous method prepares proof inputs for a transaction.
   * - It starts by shuffling the UTXOs (Unspent Transaction Outputs) for both inputs and outputs.
   *
   * - The method then fetches the transaction integrity hash and ensures it is defined, otherwise throws an error.
   * - It then proceeds to generate Merkle proofs for the input UTXOs and uses these proofs along with other transaction parameters to build the `proofInput` object.
   *
   * - If `appParams` are provided, the method calculates the transaction hash accordingly and adds it to the `proofInput` object.
   * - Additionally, it prepares the proofInput for the application-specific logic by incorporating `appParams.inputs` and public keys of the input UTXOs.
   *
   * - The `proofInput` object is used later on in the transaction process to generate zk-SNARK proofs.
   *
   * @throws {TransactionError} TransactionError: Throws an error if the transaction integrity hash is undefined after calling `getTxIntegrityHash`.
   * @returns {Promise<void>} Returns a Promise that resolves when the method has finished executing.
   *
   * @example
   * ```typescript
   * const transaction = new Transaction(params);
   * await transaction.compile();
   * ```
   */
  async compile() {
    this.firstPath = path.resolve(__dirname, "../../build-circuits/");

    this.shuffleUtxos(this.params.inputUtxos);
    this.shuffleUtxos(this.params.outputUtxos);

    if (!this.provider.solMerkleTree)
      throw new TransactionError(
        ProviderErrorCode.SOL_MERKLE_TREE_UNDEFINED,
        "getProofInput",
      );
    await this.params.getTxIntegrityHash(this.provider.poseidon);
    if (!this.params.txIntegrityHash)
      throw new TransactionError(
        TransactionErrorCode.TX_INTEGRITY_HASH_UNDEFINED,
        "compile",
      );

    const { inputMerklePathIndices, inputMerklePathElements } =
      Transaction.getMerkleProofs(this.provider, this.params.inputUtxos);

    this.proofInput = {
      root: this.provider.solMerkleTree.merkleTree.root(),
      inputNullifier: this.params.inputUtxos.map((x) =>
        x.getNullifier(this.provider.poseidon),
      ),
      publicAmountSpl: this.params.publicAmountSpl.toString(),
      publicAmountSol: this.params.publicAmountSol.toString(),
      publicMintPubkey: this.getMint(),
      inPrivateKey: this.params.inputUtxos?.map((x) => x.account.privkey),
      inPathIndices: inputMerklePathIndices,
      inPathElements: inputMerklePathElements,
      internalTxIntegrityHash: this.params.txIntegrityHash.toString(),
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

      this.proofInput.publicAppVerifier = hashAndTruncateToCircuit(
        TransactionParameters.getVerifierProgramId(
          this.appParams.verifierIdl,
        ).toBuffer(),
      );

      this.proofInput = {
        ...this.appParams.inputs,
        ...this.proofInput,
        inPublicKey: this.params?.inputUtxos?.map(
          (utxo) => utxo.account.pubkey,
        ),
      };
    }
  }

  /**
   * This method returns the mint of the spl token in the utxo.
   * @remark
   * - If the publicAmountSpl parameter of the transaction parameters is zero, the method returns zero.
   * - If the assetPubkeysCircuit property exists in the transaction parameters, the method returns the second item in the array.
   * @throws {TransactionError} TransactionError: When the assetPubkeysCircuit property is not available in the transaction parameters.
   * @returns {BN} Returns a Big Number (BN) instance representing the mint value of the transaction.
   */
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

  /**
   * @async
   * This method generates a proof and assigns the results to 'proofBytes' and 'publicInputs' of 'transactionInputs'.
   * It is used for system verifier programs.
   *
   * @throws {TransactionError} Will throw an error if any issue arises in the 'getProofInternal' method.
   *
   * @returns {Promise<void>} A promise that resolves when the proof generation and assignment is complete.
   *
   */
  async getProof() {
    const res = await this.getProofInternal(this.params, this.firstPath);
    this.transactionInputs.proofBytes = res.parsedProof;
    this.transactionInputs.publicInputs = res.parsedPublicInputsObject;
  }

  /**
   * @async
   * This method generates an application-specific proof and assigns the results to 'proofBytesApp' and 'publicInputsApp' of 'transactionInputs'.
   * It is used for general app verifier programs
   *
   * @throws {TransactionError} Will throw an error if 'appParams' is not defined or 'path' is not defined in 'appParams'.
   *
   * @returns {Promise<void>} A promise that resolves when the application-specific proof generation and assignment is complete.
   *
   */
  async getAppProof() {
    if (!this.appParams)
      throw new TransactionError(
        TransactionErrorCode.APP_PARAMETERS_UNDEFINED,
        "getAppProof",
      );
    if (!this.appParams.path)
      throw new TransactionError(
        TransactionErrorCode.FIRST_PATH_APP_UNDEFINED,
        "getAppProof",
        "app path is undefined it needs to be defined in appParams",
      );

    const res = await this.getProofInternal(
      this.appParams,
      this.appParams.path,
    );
    this.transactionInputs.proofBytesApp = {
      proofAApp: res.parsedProof.proofA,
      proofBApp: res.parsedProof.proofB,
      proofCApp: res.parsedProof.proofC,
    };
    this.transactionInputs.publicInputsApp = res.parsedPublicInputsObject;
  }

  /**
   * @async
   * This method generates and verifies a proof.
   * @note - The proof inputs and public inputs are stored in the application verifier program's idl.
   * @param {TransactionParameters | any} params - An object that contains parameters for the transaction.
   * @param {string} firstPath - The first path to be used by the Prover Class.
   *
   * @throws {TransactionError} TransactionError:
   * - Will throw an error if 'verifierIdl' is missing in TransactionParameters.
   * - Will throw an error if the proof generation fails.
   * - Will throw an error if the proof is invalid.
   *
   * @returns {Promise<object>} A promise that resolves to an object with 'parsedProof' and 'parsedPublicInputsObject'.
   *
   */
  async getProofInternal(
    params: TransactionParameters | any,
    firstPath: string,
  ) {
    if (!this.proofInput)
      throw new TransactionError(
        TransactionErrorCode.PROOF_INPUT_UNDEFINED,
        "getProofInternal",
      );
    if (!this.params)
      throw new TransactionError(
        TransactionErrorCode.NO_PARAMETERS_PROVIDED,
        "getProofInternal",
      );
    if (!this.params.verifierIdl)
      throw new TransactionError(
        TransactionErrorCode.NO_PARAMETERS_PROVIDED,
        "getProofInternal",
        "verifierIdl is missing in TransactionParameters",
      );
    let prover = new Prover(params.verifierIdl, firstPath);
    await prover.addProofInputs(this.proofInput);
    console.time("Proof generation + Parsing");
    try {
      var { parsedProof, parsedPublicInputs } =
        await prover.fullProveAndParse();
    } catch (error) {
      throw new TransactionError(
        TransactionErrorCode.PROOF_GENERATION_FAILED,
        "getProofInternal",
        error,
      );
    }
    console.timeEnd("Proof generation + Parsing");

    const res = await prover.verify();
    if (res !== true) {
      throw new TransactionError(
        TransactionErrorCode.INVALID_PROOF,
        "getProofInternal",
      );
    }
    const parsedPublicInputsObject =
      prover.parsePublicInputsFromArray(parsedPublicInputs);
    return { parsedProof, parsedPublicInputsObject };
  }

  /**
   * @static
   * This static method is used to generate the hash of a transaction. It is a poseidon hash that commits to all parameters contained in the shielded transaction (all commitment hashes, and the tx integrity hash).
   * - It takes the transaction parameters and a poseidon instance as arguments.
   * - It generates separate hashes for input UTXOs and output UTXOs and combines them with the txIntegrityHash to produce the transaction hash.
   * @param {TransactionParameters} params - The transaction parameters object. It should contain the inputUtxos, outputUtxos and txIntegrityHash properties.
   * @param {any} poseidon - The Poseidon hash function instance used for hashing.
   * @throws {TransactionError} TransactionError:When the txIntegrityHash property is not available in the transaction parameters.
   * @returns {string} Returns a string representing the transaction hash.
   */
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
   * @async
   * For this method the essence is that the merkle tree root index is sent to the program onchain as instruction data instead of the complete root because of two things.
   * - To check that the root exists onchain (which we can check in constant time if we know the index of the root in the root history array of the Merkle tree pda).
   * - To save 24 bytes of data in the instruction data by sending a u64 of 8 bytes instead of the root hash of 32 bytes.
   * @remark
   * - If the provider or the merkle tree are not defined in the provider.solMerkleTree, it defaults the root index to 0.
   * - Otherwise, it fetches the merkle tree account data and finds the root index from there.
   * @throws {TransactionError} TransactionError:
   * - If the root index is not found in the merkle tree account data.
   * @returns {Promise<void>} Returns a promise that resolves when the method has completed. The resolved value is undefined.
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

  // TODO: make this work for edge case of two 2 different assets plus fee asset in the same transaction
  // TODO: fix edge case of an assetpubkey being 0
  // TODO: !== !! and check non-null

  /**
   * This method computes the indices in which the asset for the UTXO is in the asset pubkeys array.
   * Using these indices, the zero-knowledge proof circuit enforces that only UTXOs containing the assets in the asset pubkeys array
   * are included in the transaction. This means that the UTXOs that are part of the transaction must correspond to the assets specified
   * in the `assetPubkeysCircuit` array.
   *
   * @param {Utxo[]} utxos - An array of UTXOs that are part of the transaction.
   * @throws {TransactionError} TransactionError: If the `assetPubkeysCircuit` property is not defined in the `params` object.
   * @returns {string[][][]} Returns a three-dimensional array of strings that represent the indices of the assets in the asset pubkeys array.
   *
   */
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
   * This method retrieves the Merkle proofs for each input UTXO where amounts are greater than 0.
   * - For input UTXOs where amounts equal 0, it returns Merkle paths with all elements equal to 0.
   * - This is important for the verification of transactions, where a Merkle proof is used to demonstrate
   * the inclusion of a transaction within a Merkle tree stored on the Solana blockchain, without revealing the entire tree.
   *
   * @param {Provider} provider - The provider instance that includes the solMerkleTree.
   * @param {Utxo[]} inputUtxos - An array of input UTXOs to retrieve the Merkle proofs for.
   * @throws {TransactionError} TransactionError: If the `solMerkleTree` is not defined in the provider object.
   * @returns {Object} Returns an object that includes two properties: `inputMerklePathIndices` and `inputMerklePathElements`.
   * `inputMerklePathIndices` is an array of strings representing the indices of the input UTXOs within the Merkle tree.
   * `inputMerklePathElements` is a two-dimensional array of strings representing the path elements of the input UTXOs within the Merkle tree.
   *
   * @example
   * ```typescript
   * const { inputMerklePathIndices, inputMerklePathElements } = Transaction.getMerkleProofs(provider, inputUtxos);
   * ```
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
              )} was not found. Was the local merkle tree synced since the utxo was inserted?`,
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

  /**
   * @description This method derives the Program Derived Address (PDA) for a signer authority by using the provided Merkle tree and verifier program public keys. The PDA serves as an account that the program itself controls, providing an additional layer of security and flexibility in Solana programs.
   *
   * @param {PublicKey} merkleTreeProgramId - The public key of the Merkle tree program.
   * @param {PublicKey} verifierProgramId - The public key of the verifier program.
   * @returns {PublicKey} Returns the derived PublicKey of the signer authority PDA.
   *
   * @example
   * ```typescript
   * const signerAuthorityPda = Transaction.getSignerAuthorityPda(merkleTreeProgramId, verifierProgramId);
   * ```
   */
  static getSignerAuthorityPda(
    merkleTreeProgramId: PublicKey,
    verifierProgramId: PublicKey,
  ) {
    return PublicKey.findProgramAddressSync(
      [merkleTreeProgramId.toBytes()],
      verifierProgramId,
    )[0];
  }

  /**
   * This method derives the Program Derived Address (PDA) for a registered verifier by using the provided Merkle tree and verifier program public keys.
   * Similar to `getSignerAuthorityPda` method, this derived address can be used by the program for additional control and security.
   *
   * @param {PublicKey} merkleTreeProgramId - The public key of the Merkle tree program.
   * @param {PublicKey} verifierProgramId - The public key of the verifier program.
   * @returns {PublicKey} Returns the derived PublicKey of the registered verifier PDA.
   *
   * @example
   * ```typescript
   * const registeredVerifierPda = Transaction.getRegisteredVerifierPda(merkleTreeProgramId, verifierProgramId);
   * ```
   */
  static getRegisteredVerifierPda(
    merkleTreeProgramId: PublicKey,
    verifierProgramId: PublicKey,
  ): PublicKey {
    return PublicKey.findProgramAddressSync(
      [verifierProgramId.toBytes()],
      merkleTreeProgramId,
    )[0];
  }

  // TODO: evaluate whether we need this function
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

  /**
   * This method sends a transaction to the Solana blockchain.
   * - If the action of the transaction parameters is not `SHIELD`, the transaction will be sent to a relayer.
   * - Otherwise, it will be sent directly using the provider's connection.
   *
   * @param {any} ix - The transaction instruction to be sent.
   * @returns {Promise<TransactionSignature | undefined>} Returns a promise that resolves to the transaction signature if the transaction was successfully sent, or `undefined` if there was an issue.
   *
   * @throws {TransactionError} TransactionError: If the provider's connection, transaction parameters, relayer, root index, or remaining accounts are not properly defined.
   *
   * @example
   * ```typescript
   * const transactionSignature = await myTransaction.sendTransaction(instruction);
   * ```
   */
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

  /**
   * Asynchronously generates an array of transaction instructions based on the provided transaction parameters.
   *
   * 1. Validates that the required properties of transactionInputs and verifier are defined.
   * 2. Retrieves ordered instruction names from the verifier program by:
   *    a. Filtering instructions based on a suffix pattern (e.g., "First", "Second", "Third", etc.).
   *    b. Sorting instructions according to the order of suffixes.
   * 3. Constructs an input object containing the necessary data for encoding.
   * 4. Iterates through the instruction names, encoding the inputs and generating transaction instructions.
   * 5. Returns an array of generated transaction instructions.
   *
   * @param {TransactionParameters} params - Object containing the required transaction parameters.
   * @returns {Promise<TransactionInstruction[]>} - Promise resolving to an array of generated transaction instructions.
   */
  async getInstructions(
    params: TransactionParameters,
  ): Promise<TransactionInstruction[]> {
    const verifierProgram = TransactionParameters.getVerifierProgram(
      params.verifierIdl,
    );
    if (!this.transactionInputs.publicInputs)
      throw new TransactionError(
        TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED,
        "getInstructions",
      );
    if (!verifierProgram)
      throw new TransactionError(
        TransactionErrorCode.VERIFIER_PROGRAM_UNDEFINED,
        "getInstructions",
      );

    const getOrderedInstructionNames = (verifierIdl: Idl) => {
      const orderedInstructionNames = verifierIdl.instructions
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
      message: this.params.message,
      ...this.transactionInputs.proofBytes,
      ...this.transactionInputs.publicInputs,
      rootIndex: this.transactionInputs.rootIndex,
      relayerFee: this.params.relayer.getRelayerFee(this.params.ataCreationFee),
      encryptedUtxos: Buffer.from(this.params.encryptedUtxos!),
    };
    if (this.appParams) {
      inputObject = {
        ...inputObject,
        ...this.appParams.inputs,
        ...this.transactionInputs.proofBytesApp,
        ...this.transactionInputs.publicInputsApp,
      };
    }

    var instructions = [];
    const instructionNames = getOrderedInstructionNames(params.verifierIdl);
    for (let i = 0; i < instructionNames.length; i++) {
      const instruction = instructionNames[i];
      const coder = new BorshAccountsCoder(params.verifierIdl);

      const accountName = "instructionData" + firstLetterToUpper(instruction);
      let inputs = createAccountObject(
        inputObject,
        params.verifierIdl.accounts!,
        accountName,
      );

      let inputsVec = await coder.encode(accountName, inputs);
      const methodName = firstLetterToLower(instruction);
      const method = verifierProgram.methods[
        methodName as keyof typeof verifierProgram.methods
      ](inputsVec).accounts({
        ...this.params.accounts,
        ...this.params.relayer.accounts,
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

  /**
   * This method prepares, sends, and then confirms a transaction on the Solana blockchain.
   * - The transaction is sent using the provider's connection and the transaction parameters.
   * - If the `appParams` property is set, it uses those parameters to get the instructions.
   *
   * @returns {Promise<TransactionSignature>} Returns a promise that resolves to the transaction signature if the transaction was successfully sent and confirmed.
   *
   * @throws {TransactionError} TransactionError: If there's an issue with sending the transaction, or if it fails to get the transaction instructions.
   *
   * @example
   * ```typescript
   * const transactionSignature = await myTransaction.sendAndConfirmTransaction();
   * ```
   */
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
  /**
   * This method creates a Solana transaction to close the state of the verifier.
   * - The transaction is sent and confirmed using the provider's wallet.
   * - The method first checks if the necessary dependencies like the provider's wallet and the transaction parameters are properly defined.
   * - If the `appParams` property is set, it uses those parameters to create the instruction.
   *
   * @note
   * This method is used by applications not the relayer.
   * @returns {Promise<TransactionSignature>} Returns a promise that resolves to the transaction signature if the transaction was successfully sent and confirmed.
   *
   * @throws {TransactionError} TransactionError: If the provider's wallet or the transaction parameters are not properly defined, or if the verifier program is undefined.
   *
   */
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
    if (!this.params.verifierIdl)
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
        await TransactionParameters.getVerifierProgram(this.params?.verifierIdl)
          .methods.closeVerifierState()
          .accounts({
            ...this.params.accounts,
          })
          .instruction(),
      );

      return await this.provider.wallet!.sendAndConfirmTransaction(transaction);
    }
  }

  /**
   * This method generates and stores PDA (Program Derived Addresses) for nullifier, leaves, and verifier state.
   * - It first validates that all necessary data is present (transaction inputs, verifier IDL, relayer, remaining accounts).
   * - It then generates PDAs for the nullifiers and output commitments (leaves) from the transaction inputs.
   * - Finally, it generates the PDA for the verifier state, using the relayer's public key and the verifier program ID.
   *
   * @throws {TransactionError} TransactionError: If any of the required properties (transaction inputs, verifier IDL, relayer, remaining accounts) are not properly defined.
   *
   * @example
   * ```typescript
   * myTransaction.getPdaAddresses();
   * ```
   */
  getPdaAddresses() {
    if (!this.transactionInputs.publicInputs)
      throw new TransactionError(
        TransactionErrorCode.PUBLIC_INPUTS_UNDEFINED,
        "getPdaAddresses",
        "",
      );
    if (!this.params.verifierIdl)
      throw new TransactionError(
        TransactionErrorCode.VERIFIER_IDL_UNDEFINED,
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

    let nullifiers = this.transactionInputs.publicInputs.inputNullifier;
    let signer = this.params.relayer.accounts.relayerPubkey;

    this.remainingAccounts.nullifierPdaPubkeys = [];
    for (var i in nullifiers) {
      this.remainingAccounts.nullifierPdaPubkeys.push({
        isSigner: false,
        isWritable: true,
        pubkey: Transaction.getNullifierPdaPublicKey(
          nullifiers[i],
          merkleTreeProgramId,
        ),
      });
    }

    this.remainingAccounts.leavesPdaPubkeys = [];

    for (
      var j = 0;
      j < this.transactionInputs.publicInputs.outputCommitment.length;
      j += 2
    ) {
      this.remainingAccounts.leavesPdaPubkeys.push({
        isSigner: false,
        isWritable: true,
        pubkey: PublicKey.findProgramAddressSync(
          [
            Buffer.from(
              Array.from(
                this.transactionInputs.publicInputs.outputCommitment[j],
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
        TransactionParameters.getVerifierProgramId(this.appParams.verifierIdl),
      )[0];
    } else {
      this.params.accounts.verifierState = PublicKey.findProgramAddressSync(
        [signer.toBytes(), utils.bytes.utf8.encode("VERIFIER_STATE")],
        this.params.verifierProgramId,
      )[0];
    }
  }

  static getNullifierPdaPublicKey(
    nullifier: number[],
    merkleTreeProgramId: PublicKey,
  ) {
    return PublicKey.findProgramAddressSync(
      [Uint8Array.from([...nullifier]), utils.bytes.utf8.encode("nf")],
      merkleTreeProgramId,
    )[0];
  }

  // TODO: use higher entropy rnds
  /**
   * Shuffles the array of Unspent Transaction Outputs (UTXOs) in-place using the Fisher-Yates (aka Knuth) algorithm.
   * @note The method is useful when you want to add an additional layer of unpredictability to the transactions.
   *
   * @param {Utxo[]} utxos - An array of unspent transaction outputs that will be shuffled.
   *
   * @throws {TransactionError} TransactionError: If the shuffle operation is not enabled.
   *
   * @returns {Utxo[]} - The shuffled array of UTXOs.
   *
   * @example
   * const shuffledUtxos = shuffleUtxos(utxosArray);
   */
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

  /**
   * This static method retrieves the Program Derived Address (PDA) that is associated with the SPL tokens within the context of the Merkle Tree Program.
   *
   * @remarks
   * - This method utilizes the findProgramAddressSync method of the PublicKey class which returns a PDA based on the provided seeds and program ID.
   * - The seed here is a UTF-8 encoded string "spl".
   *
   * @returns {PublicKey} The PDA related to the SPL tokens within the context of the Merkle Tree Program.
   */
  static getTokenAuthority(): PublicKey {
    return PublicKey.findProgramAddressSync(
      [utils.bytes.utf8.encode("spl")],
      merkleTreeProgramId,
    )[0];
  }
}
