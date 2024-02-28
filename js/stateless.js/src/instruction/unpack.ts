import { PublicKey } from "@solana/web3.js";
import { Tlv, TlvDataElement } from "../state/utxo-data";
import {
  MerkleContext,
  Utxo,
  UtxoWithMerkleContext,
  createUtxo,
} from "../state";

type u64 = bigint;
type InputUtxoSerial = {
  owner: number;
  leafIndex: number;
  lamports: number;
  data: TlvSerial | null;
};

type OutputUtxoSerial = {
  owner: number;
  lamports: number;
  data: TlvSerial | null;
};

type TlvSerial = TlvDataElementSerial[];

type TlvDataElementSerial = {
  discriminator: number[];
  owner: number;
  data: number[];
  dataHash: number[];
};

export type PackedInstruction = {
  validityProof: Uint8Array;
  publicKeyArray: PublicKey[];
  u64Array: BigUint64Array;
  inputUtxos: InputUtxoSerial[];
  outputUtxos: OutputUtxoSerial[];
};

function packInstruction({
    validityProof,
    publicKeyArray,
    u64Array,
    inputUtxos,
    outputUtxos,
    }: PackedInstruction): InstructionPacker {
    return {
        validityProof,
        publicKeyArray,
        u64Array,
        inputUtxos,
        outputUtxos,
    };
}){



}

function packOutUtxos() {}

/// TODO: add hashing
function unpackOutputUtxosWithMerkleContext(
  packer: InstructionPacker,
  accounts: PublicKey[]
  //   merkleTreeAccounts: PublicKey[],
  //   leafIndices: number[]
): Utxo[] {
  const outUtxos: Utxo[] = [];
  packer.outputUtxos.forEach((outUtxo, i) => {
    const ownerIndex =
      outUtxo.owner < accounts.length
        ? outUtxo.owner
        : outUtxo.owner - accounts.length;
    const owner =
      ownerIndex < accounts.length
        ? accounts[ownerIndex]
        : packer.publicKeyArray[ownerIndex - accounts.length];
    const lamports = packer.u64Array[outUtxo.lamports];
    const data = outUtxo.data
      ? unpackTlv(outUtxo.data, [...accounts, ...packer.publicKeyArray])
      : undefined;

    // const merkleCtx: MerkleContext = {
    //   hash: merkleTreeAccounts[i],
    //   merkleTree: merkleTreeAccounts[i],
    //   leafIndex: leafIndices[i],
    // };
    const utxo = createUtxo(owner, lamports, data);
    outUtxos.push(utxo);
  });
  return outUtxos;
}

function unpackInputUtxos(
  inputUtxos: InputUtxoSerial[],
  accounts: PublicKey[],
  merkleTreeAccounts: PublicKey[],
  publicKeyArray: PublicKey[],
  u64Array: number[]
): Utxo[] {
  const inUtxos: Utxo[] = [];

  inputUtxos.forEach((inUtxo, i) => {
    const ownerIndex =
      inUtxo.owner < accounts.length
        ? inUtxo.owner
        : inUtxo.owner - accounts.length;
    const owner =
      ownerIndex < accounts.length
        ? accounts[ownerIndex]
        : publicKeyArray[ownerIndex - accounts.length];
    const lamports = u64Array[inUtxo.lamports];
    let data = null; // Replace with actual deserialization logic for `inUtxo.data`

    const utxo = createUtxo(owner, lamports, data);
    // Assuming updateBlinding is a method of Utxo or a utility function
    // updateBlinding(utxo, merkleTreeAccounts[i], inUtxo.leafIndex);

    inUtxos.push(utxo);
  });

  return inUtxos;
}

function unpackTlv(tlvElements: TlvSerial, accounts: PublicKey[]): Tlv {
  const _tlvElements: TlvDataElement[] = [];
  for (const tlvElement of tlvElements) {
    const owner = accounts[tlvElement.owner];
    _tlvElements.push({
      discriminator: new Uint8Array(tlvElement.discriminator),
      owner,
      data: new Uint8Array([...tlvElement.data]),
      dataHash: new Uint8Array([...tlvElement.dataHash]),
    });
  }
  return _tlvElements;
}

function packTlv(
  tlv: Tlv,
  pubkeyArray: PublicKey[],
  accounts: PublicKey[]
): TlvSerial {
  let tlvElementsSerial: TlvDataElementSerial[] = [];

  tlv.forEach((tlvElement) => {
    // Try to find the owner in the accounts array.
    let ownerIndex = accounts.findIndex((p) => p === tlvElement.owner);
    if (ownerIndex === -1) {
      ownerIndex = pubkeyArray.findIndex((p) => p === tlvElement.owner);
      if (ownerIndex === -1) {
        // Owner not found, append to pubkeyArray and use new index
        pubkeyArray.push(tlvElement.owner);
        ownerIndex = accounts.length + pubkeyArray.length - 1;
      } else {
        // Owner found in pubkeyArray, adjust index to account for accounts length
        ownerIndex += accounts.length;
      }
    }

    const serial: TlvDataElementSerial = {
      discriminator: Array.from(tlvElement.discriminator),
      owner: ownerIndex,
      data: Array.from(tlvElement.data),
      dataHash: Array.from(tlvElement.dataHash),
    };

    tlvElementsSerial.push(serial);
  });

  return tlvElementsSerial;
}
