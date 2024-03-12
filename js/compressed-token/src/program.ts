import {
  PublicKey,
  Keypair,
  Connection,
  TransactionInstruction,
  SystemProgram,
} from '@solana/web3.js';
import { BN, Program, AnchorProvider, setProvider } from '@coral-xyz/anchor';
import { IDL, PspCompressedToken } from './idl/psp_compressed_token';
import {
  LightSystemProgram,
  UtxoWithMerkleContext,
  UtxoWithMerkleProof,
  bn,
  confirmConfig,
  defaultStaticAccountsStruct,
  toArray,
  useWallet,
} from '@lightprotocol/stateless.js';
import {
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  createInitializeMint2Instruction,
} from '@solana/spl-token';
import { MINT_AUTHORITY_SEED, POOL_SEED } from './constants';
import { Buffer } from 'buffer';

export type CompressedTransferParams = {
  /** Utxos with lamports to spend as transaction inputs */
  fromBalance: // TODO: selection upfront
  | UtxoWithMerkleContext
    | UtxoWithMerkleProof
    | (UtxoWithMerkleContext | UtxoWithMerkleProof)[];
  /** Solana Account that will receive transferred compressed lamports as utxo  */
  toPubkey: PublicKey;
  /** Amount of compressed lamports to transfer */
  amount: number | BN;
  /** Mint public key */
  mint: PublicKey;
  // /** TODO: Optional: if different feepayer than owner of utxos */
  // payer?: PublicKey;
};

/** Create Mint account for compressed Tokens */
export type CreateMintParams = {
  /** Tx feepayer */
  feePayer: PublicKey;
  /** Mint authority */
  authority: PublicKey;
  /** Mint public key */
  mint: PublicKey;
  /** Mint decimals */
  decimals: number;
  /** Optional: freeze authority */
  freezeAuthority: PublicKey | null;
  /** lamport amount for mint account rent exemption */
  rentExemptBalance: number;
};

/**
 * Create compressed token accounts
 */
export type MintToParams = {
  /** Tx feepayer */
  feePayer: PublicKey;
  /** Mint authority */
  authority: PublicKey;
  /** Mint public key */
  mint: PublicKey;
  /** The Solana Public Keys to mint to. Accepts batches */
  toPubkey: PublicKey[] | PublicKey;
  /** The amount of compressed tokens to mint. Accepts batches */
  amount: BN | BN[] | number | number[]; // TODO: check if considers mint decimals
  /** Public key of the state tree to mint into. */
  merkleTree: PublicKey; // TODO: make optional with default system state trees
};

export class CompressedTokenProgram {
  /**
   * @internal
   */
  constructor() {}

  /**
   * Public key that identifies the CompressedPda program
   */
  static programId: PublicKey = new PublicKey(
    // TODO: can add check to ensure its consistent with the idl
    '9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE',
  );

  private static _program: Program<PspCompressedToken> | null = null;

  static get program(): Program<PspCompressedToken> {
    if (!this._program) {
      this.initializeProgram();
    }
    return this._program!;
  }

  /**
   * Initializes the program statically if not already initialized.
   */
  private static initializeProgram() {
    if (!this._program) {
      const mockKeypair = Keypair.generate();
      const mockConnection = new Connection(
        'http://localhost:8899',
        'confirmed',
      );
      const mockProvider = new AnchorProvider(
        mockConnection,
        useWallet(mockKeypair),
        confirmConfig,
      );
      setProvider(mockProvider);
      this._program = new Program(IDL, this.programId, mockProvider);
    }
  }

  static deriveMintAuthorityPda = (
    authority: PublicKey,
    mint: PublicKey,
  ): PublicKey => {
    const [pubkey] = PublicKey.findProgramAddressSync(
      [MINT_AUTHORITY_SEED, authority.toBuffer(), mint.toBuffer()],
      this.programId,
    );
    return pubkey;
  };

  static deriveTokenPoolPda(mint: PublicKey): PublicKey {
    const seeds = [POOL_SEED, mint.toBuffer()];
    const [address, _] = PublicKey.findProgramAddressSync(
      seeds,
      this.programId,
    );
    return address;
  }

  static get cpiAuthorityPda(): PublicKey {
    const [address, _] = PublicKey.findProgramAddressSync(
      [Buffer.from('cpi_authority')],
      this.programId,
    );
    return address;
  }

  static async createMint(
    params: CreateMintParams,
  ): Promise<TransactionInstruction[]> {
    const { mint, authority, feePayer, rentExemptBalance } = params;

    const createMintAccountInstruction = SystemProgram.createAccount({
      fromPubkey: feePayer,
      lamports: rentExemptBalance,
      newAccountPubkey: mint,
      programId: TOKEN_PROGRAM_ID,
      space: MINT_SIZE,
    });

    const mintAuthorityPda = this.deriveMintAuthorityPda(authority, mint);

    const initializeMintInstruction = createInitializeMint2Instruction(
      mint,
      params.decimals,
      mintAuthorityPda,
      params.freezeAuthority,
      TOKEN_PROGRAM_ID,
    );

    const fundAuthorityPdaInstruction = SystemProgram.transfer({
      fromPubkey: feePayer,
      toPubkey: mintAuthorityPda,
      lamports: rentExemptBalance, // TODO: check that this is the right PDA size
    });

    const tokenPoolPda = this.deriveTokenPoolPda(mint);

    const ix = await this.program.methods
      .createMint()
      .accounts({
        mint,
        feePayer,
        authority,
        tokenPoolPda,
        systemProgram: SystemProgram.programId,
        mintAuthorityPda,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();

    return [
      createMintAccountInstruction,
      initializeMintInstruction,
      fundAuthorityPdaInstruction,
      ix,
    ];
  }

  static async mintTo(params: MintToParams): Promise<TransactionInstruction> {
    const systemKeys = defaultStaticAccountsStruct();

    const { mint, feePayer, authority, merkleTree, toPubkey, amount } = params;

    const tokenPoolPda = this.deriveTokenPoolPda(mint);
    const mintAuthorityPda = this.deriveMintAuthorityPda(authority, mint);

    const amounts = toArray<BN | number>(amount).map((amount) => bn(amount));

    const toPubkeys = toArray(toPubkey);

    const ix = await this.program.methods
      .mintTo(toPubkeys, amounts)
      .accounts({
        feePayer,
        authority,
        mintAuthorityPda,
        mint,
        tokenPoolPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        compressedPdaProgram: LightSystemProgram.programId,
        registeredProgramPda: systemKeys.registeredProgramPda,
        noopProgram: systemKeys.noopProgram,
        pspAccountCompressionAuthority:
          systemKeys.pspAccountCompressionAuthority,
        accountCompressionProgram: systemKeys.accountCompressionProgram,
        merkleTree,
      })
      .instruction();

    return ix;
  }
}
