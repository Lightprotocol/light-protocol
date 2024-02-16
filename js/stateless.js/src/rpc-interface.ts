import { PublicKey } from "@solana/web3.js";

export interface CompressedAccountInfoRpcResponse {
  compressedAccountInfo: string;
}

export interface CompressionApiInterface {
  getCompressedAccountInfo(
    assetId: PublicKey
  ): Promise<CompressedAccountInfoRpcResponse>;
  getCompressedProgramAccounts(
    assetId: PublicKey
  ): Promise<CompressedAccountInfoRpcResponse[]>;
}
