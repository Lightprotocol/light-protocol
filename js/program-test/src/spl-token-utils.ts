/**
 * SPL Token test utilities for LiteSVM
 * Provides helper functions that work directly with LiteSVM for testing SPL token operations
 */

import {
  PublicKey,
  Transaction,
  VersionedTransaction,
  SystemProgram,
  Signer,
  Keypair,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  MINT_SIZE,
  getAssociatedTokenAddressSync,
  createInitializeMint2Instruction,
  createAssociatedTokenAccountInstruction,
  createMintToInstruction,
  createTransferCheckedInstruction,
  getMinimumBalanceForRentExemptMint,
  AccountLayout,
  MintLayout,
} from "@solana/spl-token";
import { Rpc } from "@lightprotocol/stateless.js";

/**
 * Create a new SPL token mint using LiteSVM
 */
export async function splCreateMint(
  rpc: Rpc,
  payer: Signer,
  mintAuthority: PublicKey,
  freezeAuthority: PublicKey | null,
  decimals: number,
  keypair = Keypair.generate(),
  programId = TOKEN_PROGRAM_ID,
): Promise<PublicKey> {
  const lamports = await getMinimumBalanceForRentExemptMint(rpc);

  const transaction = new Transaction().add(
    SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: keypair.publicKey,
      space: MINT_SIZE,
      lamports,
      programId,
    }),
    createInitializeMint2Instruction(
      keypair.publicKey,
      decimals,
      mintAuthority,
      freezeAuthority,
      programId,
    ),
  );

  // Get blockhash and sign
  const { blockhash } = await rpc.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.sign(payer, keypair);

  // Send transaction using LiteSVM
  // Cast to VersionedTransaction since Rpc interface only accepts that type
  // but LiteSVMRpc.sendTransaction actually accepts both Transaction and VersionedTransaction
  await rpc.sendTransaction(transaction as any);

  return keypair.publicKey;
}

/**
 * Create an associated token account using LiteSVM
 */
export async function splCreateAssociatedTokenAccount(
  rpc: Rpc,
  payer: Signer,
  mint: PublicKey,
  owner: PublicKey,
  programId = TOKEN_PROGRAM_ID,
): Promise<PublicKey> {
  const associatedToken = getAssociatedTokenAddressSync(
    mint,
    owner,
    false,
    programId,
    ASSOCIATED_TOKEN_PROGRAM_ID,
  );

  const transaction = new Transaction().add(
    createAssociatedTokenAccountInstruction(
      payer.publicKey,
      associatedToken,
      owner,
      mint,
      programId,
      ASSOCIATED_TOKEN_PROGRAM_ID,
    ),
  );

  // Get blockhash and sign
  const { blockhash } = await rpc.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.sign(payer);

  // Send transaction using LiteSVM
  // Cast to VersionedTransaction since Rpc interface only accepts that type
  // but LiteSVMRpc.sendTransaction actually accepts both Transaction and VersionedTransaction
  await rpc.sendTransaction(transaction as any);

  return associatedToken;
}

/**
 * Mint tokens to an account using LiteSVM
 */
export async function splMintTo(
  rpc: Rpc,
  payer: Signer,
  mint: PublicKey,
  destination: PublicKey,
  authority: Signer,
  amount: number | bigint,
  programId = TOKEN_PROGRAM_ID,
): Promise<string> {
  const transaction = new Transaction().add(
    createMintToInstruction(
      mint,
      destination,
      authority.publicKey,
      amount,
      [],
      programId,
    ),
  );

  // Get blockhash and sign
  const { blockhash } = await rpc.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.sign(payer, authority);

  // Send transaction using LiteSVM
  // Cast to VersionedTransaction since Rpc interface only accepts that type
  // but LiteSVMRpc.sendTransaction actually accepts both Transaction and VersionedTransaction
  return rpc.sendTransaction(transaction as any);
}

/**
 * Transfer tokens between accounts using LiteSVM
 */
export async function splTransfer(
  rpc: Rpc,
  payer: Signer,
  source: PublicKey,
  mint: PublicKey,
  destination: PublicKey,
  owner: Signer,
  amount: number | bigint,
  decimals: number,
  programId = TOKEN_PROGRAM_ID,
): Promise<string> {
  const transaction = new Transaction().add(
    createTransferCheckedInstruction(
      source,
      mint,
      destination,
      owner.publicKey,
      amount,
      decimals,
      [],
      programId,
    ),
  );

  // Get blockhash and sign
  const { blockhash } = await rpc.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.sign(payer, owner);

  // Send transaction using LiteSVM
  return rpc.sendTransaction(transaction as any);
}

/**
 * Get token account balance
 */
export async function splGetTokenAccountBalance(
  rpc: Rpc,
  tokenAccount: PublicKey,
): Promise<bigint> {
  const accountInfo = await rpc.getAccountInfo(tokenAccount);

  if (!accountInfo) {
    throw new Error("Token account not found");
  }

  const data = AccountLayout.decode(accountInfo.data);
  console.log(
    "[spl-token-utils.ts:195] Converting amount:",
    typeof data.amount,
    data.amount,
  );
  const amount =
    typeof data.amount === "bigint"
      ? data.amount
      : BigInt((data.amount as any).toString());
  return amount;
}

/**
 * Get mint info
 */
export async function splGetMintInfo(
  rpc: Rpc,
  mint: PublicKey,
): Promise<{
  mintAuthority: PublicKey | null;
  supply: bigint;
  decimals: number;
  isInitialized: boolean;
  freezeAuthority: PublicKey | null;
}> {
  const accountInfo = await rpc.getAccountInfo(mint);

  if (!accountInfo) {
    throw new Error("Mint not found");
  }

  const data = MintLayout.decode(accountInfo.data);
  console.log(
    "[spl-token-utils.ts:223] Converting supply:",
    typeof data.supply,
    data.supply,
  );

  const supply =
    typeof data.supply === "bigint"
      ? data.supply
      : BigInt((data.supply as any).toString());
  const isInitialized =
    typeof data.isInitialized === "boolean"
      ? data.isInitialized
      : data.isInitialized !== 0;

  return {
    mintAuthority:
      data.mintAuthorityOption === 0 ? null : new PublicKey(data.mintAuthority),
    supply,
    decimals: data.decimals,
    isInitialized,
    freezeAuthority:
      data.freezeAuthorityOption === 0
        ? null
        : new PublicKey(data.freezeAuthority),
  };
}

/**
 * Check if a token account exists
 */
export async function splTokenAccountExists(
  rpc: Rpc,
  tokenAccount: PublicKey,
): Promise<boolean> {
  const accountInfo = await rpc.getAccountInfo(tokenAccount);
  return accountInfo !== null;
}

/**
 * Get or create an associated token account
 * Replicates the behavior of getOrCreateAssociatedTokenAccount from @solana/spl-token
 */
export async function splGetOrCreateAssociatedTokenAccount(
  rpc: Rpc,
  payer: Signer,
  mint: PublicKey,
  owner: PublicKey,
  allowOwnerOffCurve = false,
  commitment?: any,
  confirmOptions?: any,
  programId = TOKEN_PROGRAM_ID,
  associatedTokenProgramId = ASSOCIATED_TOKEN_PROGRAM_ID,
): Promise<{ address: PublicKey; isNew: boolean }> {
  const associatedToken = getAssociatedTokenAddressSync(
    mint,
    owner,
    allowOwnerOffCurve,
    programId,
    associatedTokenProgramId,
  );

  // Check if the account exists
  const accountInfo = await rpc.getAccountInfo(associatedToken);

  if (accountInfo !== null) {
    // Account already exists
    return { address: associatedToken, isNew: false };
  }

  // Create the account
  const transaction = new Transaction().add(
    createAssociatedTokenAccountInstruction(
      payer.publicKey,
      associatedToken,
      owner,
      mint,
      programId,
      associatedTokenProgramId,
    ),
  );

  // Get blockhash and sign
  const { blockhash } = await rpc.getLatestBlockhash();
  transaction.recentBlockhash = blockhash;
  transaction.sign(payer);

  // Send transaction using LiteSVM
  await rpc.sendTransaction(transaction as any);

  return { address: associatedToken, isNew: true };
}
