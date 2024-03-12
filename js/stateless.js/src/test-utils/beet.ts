import { PublicKey } from '@solana/web3.js';

import {
  array,
  coption,
  fixedSizeUint8Array,
  u64,
  FixableBeetStruct,
  bignum,
  u8,
} from '@metaplex-foundation/beet';

import { publicKey } from '@metaplex-foundation/beet-solana';

export class ParsingTlvElementBeet {
  constructor(
    readonly discriminator: Uint8Array,
    readonly owner: PublicKey,
    readonly data: number[],
    readonly dataHash: Uint8Array,
  ) {}
  static readonly struct = new FixableBeetStruct<
    ParsingTlvElementBeet,
    ParsingTlvElementBeet
  >(
    [
      ['discriminator', fixedSizeUint8Array(8)],
      ['owner', publicKey],
      ['data', array(u8)],
      ['dataHash', fixedSizeUint8Array(32)],
    ],
    (args) =>
      new ParsingTlvElementBeet(
        args.discriminator,
        args.owner,
        args.data,
        args.dataHash,
      ),
    'ParsingTlvElementBeet',
  );
}

export class ParsingTlvBeet {
  constructor(readonly tlvElements: ParsingTlvElementBeet[] | null) {}

  static readonly struct = new FixableBeetStruct<
    ParsingTlvBeet,
    ParsingTlvBeet
  >(
    [['tlvElements', array(ParsingTlvElementBeet.struct)]],
    (args) => new ParsingTlvBeet(args.tlvElements),
    'ParsingTlvBeet',
  );
}

export class ParsingUtxoBeet {
  constructor(
    readonly owner: PublicKey,
    readonly blinding: Uint8Array,
    readonly lamports: bignum,
    readonly data: ParsingTlvBeet | null,
  ) {}

  static readonly struct = new FixableBeetStruct<
    ParsingUtxoBeet,
    ParsingUtxoBeet
  >(
    [
      ['owner', publicKey],
      ['blinding', fixedSizeUint8Array(32)],
      ['lamports', u64],
      ['data', coption(ParsingTlvBeet.struct)],
    ],
    (args) =>
      new ParsingUtxoBeet(args.owner, args.blinding, args.lamports, args.data),
    'ParsingUtxo',
  );
}

export class PublicTransactionIndexerEventBeet {
  constructor(
    readonly inUtxos: ParsingUtxoBeet[],
    readonly outUtxos: ParsingUtxoBeet[],
    readonly outUtxoIndices: bignum[],
    readonly deCompressAmount: bignum | null,
    readonly relayFee: bignum | null,
    readonly message: number[] | null,
  ) {}

  static readonly struct = new FixableBeetStruct<
    PublicTransactionIndexerEventBeet,
    PublicTransactionIndexerEventBeet
  >(
    [
      ['inUtxos', array(ParsingUtxoBeet.struct)],
      ['outUtxos', array(ParsingUtxoBeet.struct)],
      ['outUtxoIndices', array(u64)],
      ['deCompressAmount', coption(u64)],
      ['relayFee', coption(u64)],
      ['message', coption(array(u8))],
    ],
    (args) =>
      new PublicTransactionIndexerEventBeet(
        args.inUtxos,
        args.outUtxos,
        args.outUtxoIndices,
        args.deCompressAmount,
        args.relayFee,
        args.message,
      ),
    'PublicTransactionIndexerEvent',
  );
}
