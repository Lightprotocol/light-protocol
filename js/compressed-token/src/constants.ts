import { utils } from '@coral-xyz/anchor';

export const POOL_SEED = Buffer.from('pool');

export const MINT_AUTHORITY_SEED =
  utils.bytes.utf8.encode('mint_authority_pda');

export const SPL_TOKEN_MINT_RENT_EXEMPT_BALANCE = 1461600;
