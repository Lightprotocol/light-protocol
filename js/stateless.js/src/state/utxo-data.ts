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

/** Factory for TLV data elements */
export const createTlvDataElement = (
  discriminator: Uint8Array,
  owner: PublicKey,
  data: Uint8Array,
  dataHash: Uint8Array
): TlvDataElement => ({
  discriminator,
  owner,
  data,
  dataHash,
});

const { coder } = LightSystemProgram.program;

/** Decode system-level utxo data into tlvs from a buffer */
export function decodeUtxoData(buffer: Buffer): TlvDataElement[] {
  return coder.types.decode("Tlv", buffer);
}

/** Encode tlv blocks into a buffer */
export function encodeUtxoData(data: TlvDataElement[]): Buffer {
  return coder.types.encode("Tlv", data);
}

export const isValidTlvDataElement = (value: any): value is TlvDataElement => {
  if (!value) return false;
  if (typeof value !== "object") return false;
  if (!(value.discriminator instanceof Uint8Array)) return false;
  if (!(value.owner instanceof PublicKey)) return false; // Assuming PublicKey is a class or constructor function
  if (!(value.data instanceof Uint8Array)) return false;
  if (!(value.dataHash instanceof Uint8Array)) return false;
  return true;
};
