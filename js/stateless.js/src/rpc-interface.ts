import { PublicKey } from "@solana/web3.js";

export interface CompressedAccountInfoRpcResponse {
  compressedAccountInfo: string;
}

export interface CompressionApiInterface {
  /// TODO: add all rpc methods

  getCompressedAccountInfo(
    assetId: PublicKey
  ): Promise<CompressedAccountInfoRpcResponse>;
  getCompressedProgramAccounts(
    assetId: PublicKey
  ): Promise<CompressedAccountInfoRpcResponse[]>;
}
