import * as anchor from "@project-serum/anchor";
import { Connection, Keypair } from "@solana/web3.js";
export declare const newAccountWithLamports: (
  connection: any,
  account?: anchor.web3.Account,
  lamports?: number
) => Promise<anchor.web3.Account>;
export declare const newAddressWithLamports: (
  connection: any,
  address?: anchor.web3.PublicKey,
  lamports?: number
) => Promise<anchor.web3.PublicKey>;
export declare const newProgramOwnedAccount: ({
  connection,
  owner,
  lamports,
}: {
  connection: any;
  owner: any;
  lamports?: number | undefined;
}) => Promise<anchor.web3.Account>;
export declare function newAccountWithTokens({
  connection,
  MINT,
  ADMIN_AUTH_KEYPAIR,
  userAccount,
  amount,
}: {
  connection: any;
  MINT: any;
  ADMIN_AUTH_KEYPAIR: any;
  userAccount: any;
  amount: any;
}): Promise<any>;
export declare function createMintWrapper({
  authorityKeypair,
  mintKeypair,
  nft,
  decimals,
  connection,
}: {
  authorityKeypair: Keypair;
  mintKeypair: Keypair;
  nft: Boolean;
  decimals: number;
  connection: Connection;
}): Promise<anchor.web3.PublicKey | undefined>;
export declare function createTestAccounts(connection: Connection): Promise<{
  POSEIDON: any;
  KEYPAIR: anchor.web3.Keypair;
  RELAYER_RECIPIENT: anchor.web3.PublicKey;
}>;
