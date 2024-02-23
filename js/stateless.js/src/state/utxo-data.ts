import { PublicKey } from "@solana/web3.js";
import { LightSystemProgram } from "../programs/compressed-pda";
import { Buffer } from "buffer";

/** Describe the generic details applicable to every data block */
export type TlvDataElement = {
  discriminator: Uint8Array;
  /** Public key of the ownerProgram of the data block */
  owner: PublicKey;
  /** Variable-length data */
  data: Uint8Array;
  /** Poseidon hash of data */
  dataHash: Uint8Array; // Consider using bigint254
};

const { coder } = LightSystemProgram.program;

/** Decode system-level utxo data into tlvs from a buffer */
export function decodeUtxoData(buffer: Buffer): TlvDataElement[] {
  return coder.types.decode("Tlv", buffer);
}

/** Encode tlv blocks into a buffer */
export function encodeUtxoData(data: TlvDataElement[]): Buffer {
  return coder.types.encode("Tlv", data);
}
