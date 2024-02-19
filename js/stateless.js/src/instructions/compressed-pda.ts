import { Program, AnchorProvider, setProvider } from "@coral-xyz/anchor";
import {
  PublicKey,
  TransactionInstruction,
  Keypair,
  Connection,
} from "@solana/web3.js";
import { IDL, PspCompressedPda } from "../idls/psp_compressed_pda";
import { confirmConfig } from "../constants";
import { useWallet } from "../wallet";
import { UtxoWithMerkleProof } from "../state";

export type CompressedTransferParams = {
  /** Utxos with lamports to spend as transaction inputs */
  fromBalance: UtxoWithMerkleProof[];
  /** Solana Account that will receive transferred compressed lamports as utxo  */
  toPubkey: PublicKey;
  /** Amount of compressed lamports to transfer */
  lamports: number | bigint;
};

/**
 * Create compressed account system transaction params
 */
export type CreateCompressedAccountParams = {
  /**
   * Optional utxos with lamports to spend as transaction inputs.
   * Not required unless 'lamports' are specified, as Light doesn't
   * enforce rent on the protocol level.
   * */
  fromBalance: UtxoWithMerkleProof[];
  /** Public key of the created account */
  newAccountPubkey: PublicKey;
  /** Amount of lamports to transfer to the created compressed account */
  lamports: number | bigint;
  /** Public key of the program to assign as the owner of the created compressed account */
  programId: PublicKey;
};

/**
 * Example usage:
 * ```typescript
 * const tx = await LightSystemProgram.transfer({
 *  fromBalance: [
 *   {
 *      owner: new PublicKey("..."),
 *      data: [],
 *      lamports: 2000000000n,
 *      leafIndex: 0n,
 *      hash: 0n,
 *      merkletreeId: 0n,
 *      merkleProof: [],
 *   },
 * ],
 * toPubkey: new PublicKey("..."),
 * lamports: 1000000000n,
 * });
 * ```
 */
export class LightSystemProgram {
  /**
   * @internal
   */
  constructor() {}

  /**
   * Public key that identifies the CompressedPda program
   */
  static programId: PublicKey = new PublicKey(
    // TODO: replace with actual program id
    // can add check to ensure its consistent with the idl
    "11111111111111111111111111111111"
  );

  private static _program: Program<PspCompressedPda> | null = null;

  /**
   * Accessor for the program. Ensures the program is initialized before returning.
   */
  static get program(): Program<PspCompressedPda> {
    if (!this._program) {
      this.initializeProgram();
    }
    return this._program!;
  }

  /**
   * Initializes the program statically if not already initialized.
   * This method can be called to ensure the program is ready for use.
   */
  private static initializeProgram() {
    if (!this._program) {
      const mockKeypair = Keypair.generate();
      const mockConnection = new Connection(
        "http://localhost:8899",
        "confirmed"
      );
      const mockProvider = new AnchorProvider(
        mockConnection,
        useWallet(mockKeypair),
        confirmConfig
      );
      setProvider(mockProvider);
      this._program = new Program(IDL, this.programId, mockProvider);
    }
  }

  /**
   * Generate a transaction instruction that transfers compressed
   * lamports from one compressed balance to a solana address
   */
  static async transfer(
    params: CompressedTransferParams
  ): Promise<TransactionInstruction> {
    this.initializeProgram();

    /// TODO: call the packer function with the transfer params instead
    /// const systemKeys = getLightSystemProgramAccountKeys() /// this should return the respective lookupTable too. this could include: const mtKeys = getLightSystemProgramActiveMerkleTreeKeys()
    /// const outputUtxos = createTxOutputUtxosForTransfer(params.fromBalance, params.lamports, params.toPubkey)
    /// const {data, keys} = pack(systemKeys, mtKeys, outputUtxos) /// efficiently packs ixData
    const data = Buffer.from([]) as any;
    const keys = [] as any;

    const ix = await this._program.methods
      .executeCompressedTransaction2(data)
      .accounts(keys)
      .instruction();

    return ix;
  }

  /**
   * Generate a transaction instruction that creates a new account
   */
  //   static createAccount(params: CreateAccountParams): TransactionInstruction {

  //   }
}
