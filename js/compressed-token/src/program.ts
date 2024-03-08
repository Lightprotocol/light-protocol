import {
  PublicKey,
  Keypair,
  Connection,
  TransactionInstruction,
  SystemProgram,
} from '@solana/web3.js';
import {
  BN,
  Program,
  AnchorProvider,
  setProvider,
  utils,
} from '@coral-xyz/anchor';
import { IDL, PspCompressedToken } from './idl/psp_compressed_token';
import {
  LightSystemProgram,
  Utxo,
  UtxoWithMerkleContext,
  UtxoWithMerkleProof,
  addMerkleContextToUtxo,
  bn,
  coerceIntoUtxoWithMerkleContext,
  confirmConfig,
  createUtxo,
  defaultStaticAccounts,
  defaultStaticAccountsStruct,
  merkleTreeProgramId,
  packInstruction,
  pipe,
  placeholderValidityProof,
  toArray,
  useWallet,
} from '@lightprotocol/stateless.js';
import {
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  createInitializeMint2Instruction,
} from '@solana/spl-token';
import { POOL_SEED_BYTES } from './constants';

/** In order to reduce rpc roundtrips we can hardcode the minimum_rent_exemption
 * for spl token mints
 * MINT_SIZE is = 82 bytes = 1461600 lamports
 *    export const MintLayout = struct<RawMint>([
 *      u32('mintAuthorityOption'), // 4 bytes publicKey('mintAuthority'),
 *      // 32 bytes u64('supply'), // 8 bytes u8('decimals'), // 1 byte
 *      bool('isInitialized'), // 1 byte u32('freezeAuthorityOption'), // 4
 *      bytes publicKey('freezeAuthority'), // 32 bytes
 *    ]);
 */

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
  mint: PublicKey;
  // TODO: add
  // /** Optional: if different feepayer than owner of utxos */
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
      [
        utils.bytes.utf8.encode('authority'),
        authority.toBuffer(),
        mint.toBuffer(),
      ],
      this.programId,
    );
    return pubkey;
  };

  static deriveTokenPoolPda(mint: PublicKey): PublicKey {
    const seeds = [POOL_SEED_BYTES, mint.toBuffer()];
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
    const mint = params.mint;
    const authority = params.authority;
    const feePayer = params.feePayer;
    const rentExemptBalance = params.rentExemptBalance;

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

    const amounts = toArray(amount).map((amount) => bn(amount));

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

  /**
   * Generate a transaction instruction that transfers compressed
   * tokens from one compressed balance to another solana address
   */
  /// TODO: should just define the createoutput utxo selection + packing
  // static async transfer(
  //   params: CompressedTransferParams,
  // ): Promise<TransactionInstruction> {
  //   const recipientUtxo = createUtxo(params.toPubkey, params.lamports);

  //   // unnecessary if after
  //   const fromUtxos = pipe(
  //     toArray<UtxoWithMerkleContext | UtxoWithMerkleProof>,
  //     coerceIntoUtxoWithMerkleContext,
  //   )(params.fromBalance);

  //   // TODO: move outside of transfer, selection and (getting merkleproofs and zkp) should happen BEFORE call
  //   /// find sort utxos by size, then add utxos up until the amount is at least reached, return the selected utxos
  //   if (new Set(fromUtxos.map((utxo) => utxo.owner.toString())).size > 1) {
  //     throw new Error('All input utxos must have the same owner');
  //   }
  //   const selectedInputUtxos = fromUtxos
  //     .sort((a, b) => Number(bn(a.lamports).sub(bn(b.lamports))))
  //     .reduce<{
  //       utxos: (UtxoWithMerkleContext | UtxoWithMerkleProof)[];
  //       total: BN;
  //     }>(
  //       (acc, utxo) => {
  //         if (bn(acc.total).lt(bn(params.lamports))) {
  //           acc.utxos.push(utxo);
  //           acc.total = bn(acc.total).add(bn(utxo.lamports));
  //         }
  //         return acc;
  //       },
  //       { utxos: [], total: bn(0) },
  //     );

  //   /// transfer logic
  //   let changeUtxo;
  //   const changeAmount = bn(selectedInputUtxos.total).sub(bn(params.lamports));
  //   if (bn(changeAmount).gt(bn(0))) {
  //     changeUtxo = createUtxo(selectedInputUtxos.utxos[0].owner, changeAmount);
  //   }

  //   const outputUtxos = changeUtxo
  //     ? [recipientUtxo, changeUtxo]
  //     : [recipientUtxo];

  //   // TODO: move zkp, merkleproof generation, and rootindices outside of transfer
  //   const recentValidityProof = placeholderValidityProof();
  //   const recentInputStateRootIndices = selectedInputUtxos.utxos.map((_) => 0);
  //   const staticAccounts = defaultStaticAccounts();

  //   const ix = await packInstruction({
  //     inputState: coerceIntoUtxoWithMerkleContext(selectedInputUtxos.utxos),
  //     outputState: outputUtxos,
  //     recentValidityProof,
  //     recentInputStateRootIndices,
  //     payer: selectedInputUtxos.utxos[0].owner, // TODO: dynamic payer,
  //     staticAccounts,
  //   });
  //   return ix;
  // }
}

//@ts-ignore
if (import.meta.vitest) {
  //@ts-ignore
  const { it, expect, describe } = import.meta.vitest;

  // describe('LightSystemProgram.transfer function', () => {
  //   it('should return a transaction instruction that transfers compressed lamports from one compressed balance to another solana address', async () => {
  //     const randomPubKeys = [
  //       PublicKey.unique(),
  //       PublicKey.unique(),
  //       PublicKey.unique(),
  //       PublicKey.unique(),
  //       PublicKey.unique(), // 4th
  //     ];
  //     const fromBalance = [
  //       addMerkleContextToUtxo(
  //         createUtxo(randomPubKeys[0], bn(1)),
  //         bn(0),
  //         randomPubKeys[3],
  //         0,
  //         randomPubKeys[4],
  //       ),
  //       addMerkleContextToUtxo(
  //         createUtxo(randomPubKeys[0], bn(2)),
  //         bn(0),
  //         randomPubKeys[3],
  //         1,
  //         randomPubKeys[4],
  //       ),
  //     ];
  //     const toPubkey = PublicKey.unique();
  //     const lamports = bn(2);
  //     const ix = await LightSystemProgram.transfer({
  //       fromBalance,
  //       toPubkey,
  //       lamports,
  //     });

  //     console.log('ix', ix.data, ix.data.length);

  //     expect(ix).toBeDefined();
  //   });

  //   it('should throw an error when the input utxos have different owners', async () => {
  //     const randomPubKeys = [
  //       PublicKey.unique(),
  //       PublicKey.unique(),
  //       PublicKey.unique(),
  //       PublicKey.unique(),
  //     ];
  //     const fromBalance = [
  //       addMerkleContextToUtxo(
  //         createUtxo(randomPubKeys[0], bn(1)),
  //         bn(0),
  //         randomPubKeys[3],
  //         0,
  //         randomPubKeys[4],
  //       ),
  //       addMerkleContextToUtxo(
  //         createUtxo(randomPubKeys[1], bn(2)), // diff owner key
  //         bn(0),
  //         randomPubKeys[3],
  //         1,
  //         randomPubKeys[4],
  //       ),
  //     ];
  //     const toPubkey = PublicKey.unique();
  //     const lamports = bn(2);
  //     await expect(
  //       LightSystemProgram.transfer({
  //         fromBalance,
  //         toPubkey,
  //         lamports,
  //       }),
  //     ).rejects.toThrow('All input utxos must have the same owner');
  //   });
  // });
}
