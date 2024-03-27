import { describe, it, expect } from 'vitest';
import { LightSystemProgram } from '../../src/programs';
import {
  CompressedAccount,
  PublicTransactionEvent,
  bn,
  useWallet,
} from '../../src';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { AnchorProvider, Program, setProvider } from '@coral-xyz/anchor';
import { IDL } from '../../src/idls/account_compression';

describe('account compression program', () => {
  it('instantiate using IDL', async () => {
    const mockKeypair = Keypair.generate();
    const mockConnection = new Connection('http://127.0.0.1:8899', 'confirmed');
    const mockProvider = new AnchorProvider(
      mockConnection,
      useWallet(mockKeypair),
      {
        commitment: 'confirmed',
        preflightCommitment: 'confirmed',
      },
    );
    setProvider(mockProvider);
    const program = new Program(
      IDL,
      new PublicKey('5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN'),
      mockProvider,
    );

    expect(program).toBeDefined();
  });
});

describe('serde', () => {
  it('decode output compressed account ', async () => {
    const compressedAccount = [
      88, 8, 48, 185, 124, 227, 14, 195, 230, 152, 61, 39, 56, 191, 13, 126, 54,
      43, 47, 131, 175, 16, 52, 167, 129, 174, 200, 118, 174, 9, 254, 80, 0, 0,
      0, 0, 0, 0, 0, 0, 0, 0,
    ];

    const deserializedCompressedAccount: CompressedAccount =
      LightSystemProgram.program.coder.types.decode(
        'CompressedAccount',
        Buffer.from(compressedAccount),
      );

    expect(deserializedCompressedAccount.data).toBe(null);
    expect(deserializedCompressedAccount.address).toBe(null);
    expect(deserializedCompressedAccount.lamports.eq(bn(0))).toBe(true);
  });

  it('decode event ', async () => {
    const data = [
      0, 0, 0, 0, 1, 0, 0, 0, 21, 2, 159, 146, 115, 243, 27, 245, 225, 130, 22,
      145, 247, 216, 21, 84, 136, 140, 91, 209, 249, 136, 44, 124, 235, 209,
      230, 254, 72, 190, 187, 107, 0, 0, 0, 0, 1, 0, 0, 0, 191, 190, 219, 108,
      109, 150, 78, 142, 89, 168, 144, 217, 102, 58, 224, 64, 118, 152, 19, 51,
      97, 198, 36, 158, 140, 153, 125, 208, 187, 78, 107, 249, 0, 0, 0, 0, 0, 0,
      0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 68,
      77, 125, 32, 76, 128, 61, 180, 1, 207, 69, 44, 121, 118, 153, 17, 179,
      183, 115, 34, 163, 127, 102, 214, 1, 87, 175, 177, 95, 49, 65, 69, 0,
    ];
    const event: PublicTransactionEvent =
      LightSystemProgram.program.coder.types.decode(
        'PublicTransactionEvent',
        Buffer.from(data),
      );

    const ref = [
      21, 2, 159, 146, 115, 243, 27, 245, 225, 130, 22, 145, 247, 216, 21, 84,
      136, 140, 91, 209, 249, 136, 44, 124, 235, 209, 230, 254, 72, 190, 187,
      107,
    ];
    expect(bn(event.outputCompressedAccountHashes[0]).eq(bn(ref))).toBe(true);
  });
});
