import { PublicKey } from "@solana/web3.js";
import { LightSystemProgram } from "../programs/compressed-pda";
import { Buffer } from "buffer";
import { PublicKey254 } from "./utxo";
import { bigint254, createBigint254 } from "./bigint254";
import { arrayToBigint, bigintToArray } from "../utils/conversion";

/** Describe the generic details applicable to every data block */
export type TlvDataElement = {
  discriminator: Uint8Array;
  /** Public key of the ownerProgram of the data block */
  owner: PublicKey;
  /** Variable-length data */
  data: Uint8Array;
  /** Poseidon hash of data */
  dataHash: bigint254; // Consider using bigint254
};

export type TlvDataElementSerial = {
  discriminator: number[];
  /** Index of the owner in the pubkey_array */
  owner: number;
  data: number[];
  dataHash: number[];
};

export type Tlv = TlvDataElement[];
export type TlvSerial = TlvDataElementSerial[];

/** Factory for TLV data elements */
export const createTlvDataElement = (
  discriminator: Uint8Array,
  owner: PublicKey,
  data: Uint8Array,
  dataHash: bigint254
): TlvDataElement => ({
  discriminator,
  owner,
  data,
  dataHash,
});

const { coder } = LightSystemProgram.program;

/** Decode system-level utxo data into tlvs from a buffer */
export function decodeUtxoData(buffer: Buffer, accounts?: PublicKey[]): Tlv {
  const serial = coder.types.decode("TlvSerializable", buffer);
  // TODO: check if need to unpack
  // return deserializeTlv(serial, accounts);
  return serial;
}

/** Encode tlv blocks into a buffer  */
export function encodeUtxoData(
  data: Tlv,
  pubkeyArray: PublicKey[],
  accounts: PublicKey[]
): Buffer {
  const serial = serializeTlv(data, pubkeyArray, accounts);
  return coder.types.encode("TlvSerializable", serial);
}

export const isValidTlvDataElement = (value: any): value is TlvDataElement => {
  if (!value) return false;
  if (typeof value !== "object") return false;
  if (!(value.discriminator instanceof Uint8Array)) return false;
  if (!(value.owner instanceof PublicKey)) return false;
  if (!(value.data instanceof Uint8Array)) return false;
  if (!(value.dataHash instanceof Uint8Array)) return false;
  return true;
};

export function serializeTlv(
  tlv: Tlv,
  pubkeyArray: PublicKey[],
  accounts: PublicKey[]
): TlvSerial {
  const tlvElementsSerializable: TlvDataElementSerial[] = [];

  tlv.forEach((element) => {
    let ownerIndex = accounts.findIndex((acc) => acc.equals(element.owner));
    if (ownerIndex === -1) {
      ownerIndex = pubkeyArray.findIndex((pubkey) =>
        pubkey.equals(element.owner)
      );
      if (ownerIndex === -1) {
        pubkeyArray.push(element.owner);
        ownerIndex = accounts.length + pubkeyArray.length - 1;
      } else {
        ownerIndex += accounts.length;
      }
    }

    const serializableElement: TlvDataElementSerial = {
      discriminator: Array.from(element.discriminator),
      owner: ownerIndex,
      data: Array.from(element.data),
      dataHash: bigintToArray(element.dataHash),
    };

    tlvElementsSerializable.push(serializableElement);
  });

  return tlvElementsSerializable;
}

// TODO: check how events get emitted on-chain!
// we might not need to unpack the tlvs
export function deserializeTlv(
  serializable: TlvDataElementSerial[],
  accounts: PublicKey[]
): Tlv {
  const tlvElements: TlvDataElement[] = serializable.map((element) => {
    const owner = accounts[element.owner];
    return {
      discriminator: new Uint8Array(element.discriminator),
      owner,
      data: new Uint8Array(element.data),
      dataHash: arrayToBigint(element.dataHash),
    };
  });

  return tlvElements;
}
