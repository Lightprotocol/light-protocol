import { PublicKey } from '@solana/web3.js';
import { LightSystemProgram } from '../programs/compressed-pda';
import { Buffer } from 'buffer';
import {
  TlvDataElementSerializable_IdlType,
  TlvDataElement_IdlType,
  TlvSerializable_IdlType,
  Tlv_IdlType,
} from './types';

/** Factory for TLV data elements */
export const createTlvDataElement = (
  discriminator: number[],
  owner: PublicKey,
  data: Uint8Array,
  dataHash: number[],
): TlvDataElement_IdlType => ({
  discriminator,
  owner,
  data,
  dataHash,
});

/** Decode system-level utxo data into tlvs from a buffer */
export function decodeUtxoData(
  buffer: Buffer,
  accounts?: PublicKey[],
): Tlv_IdlType {
  const { coder } = LightSystemProgram.program;
  const serial = coder.types.decode('TlvSerializable', buffer);
  // TODO: check if need to unpack return deserializeTlv(serial, accounts);
  return serial;
}

/** Encode tlv blocks into a buffer  */
export function encodeUtxoData(
  data: Tlv_IdlType,
  pubkeyArray: PublicKey[],
  accounts: PublicKey[],
): Buffer {
  const { coder } = LightSystemProgram.program;

  const serial = serializeTlv(data, pubkeyArray, accounts);
  return coder.types.encode('TlvSerializable', serial);
}

export const isValidTlvDataElement = (
  value: any,
): value is TlvDataElement_IdlType => {
  if (!value) return false;
  if (typeof value !== 'object') return false;
  if (!(value.discriminator instanceof Uint8Array)) return false;
  if (!(value.owner instanceof PublicKey)) return false;
  if (!(value.data instanceof Uint8Array)) return false;
  if (!(value.dataHash instanceof Uint8Array)) return false;
  return true;
};

export function serializeTlv(
  tlv: Tlv_IdlType,
  pubkeyArray: PublicKey[],
  accounts: PublicKey[],
): TlvSerializable_IdlType {
  const tlvElementsSerializable: TlvDataElementSerializable_IdlType[] = [];

  tlv.tlvElements.forEach((element) => {
    let ownerIndex = accounts.findIndex((acc) => acc.equals(element.owner));
    if (ownerIndex === -1) {
      ownerIndex = pubkeyArray.findIndex((pubkey) =>
        pubkey.equals(element.owner),
      );
      if (ownerIndex === -1) {
        pubkeyArray.push(element.owner);
        ownerIndex = accounts.length + pubkeyArray.length - 1;
      } else {
        ownerIndex += accounts.length;
      }
    }

    const serializableElement: TlvDataElementSerializable_IdlType = {
      discriminator: Array.from(element.discriminator),
      owner: ownerIndex,
      data: element.data,
      dataHash: element.dataHash,
    };

    tlvElementsSerializable.push(serializableElement);
  });

  return { tlvElements: tlvElementsSerializable };
}

// TODO: check how events get emitted on-chain!
// we might not need to unpack the tlvs
export function deserializeTlv(
  serializable: TlvSerializable_IdlType,
  accounts: PublicKey[],
): Tlv_IdlType {
  const tlvElements: TlvDataElement_IdlType[] = serializable.tlvElements.map(
    (element) => {
      const owner = accounts[element.owner];
      return {
        discriminator: element.discriminator,
        owner,
        data: new Uint8Array(element.data),
        dataHash: element.dataHash,
      };
    },
  );

  return { tlvElements };
}
