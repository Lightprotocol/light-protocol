import { describe, it, expect } from 'vitest';
import { LightSystemProgram } from '../../src/programs';
import { Utxo_IdlType, bn, useWallet } from '../../src';
import { Connection, Keypair, PublicKey } from '@solana/web3.js';
import { AnchorProvider, Program, setProvider } from '@coral-xyz/anchor';
import { IDL, AccountCompression } from '../../src/idls/account_compression';

describe.skip('ACP test', () => {
  it('serialize compressed account', async () => {
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
    const ACP = new Program(
      IDL,
      new PublicKey('5QPEJ5zDsVou9FQS3KCauKswM3VwBEBu4dpL9xTqkWwN'),
      mockProvider,
    );
  });
});

describe.skip('Serialization test', () => {
  it('serialize utxo ', async () => {
    const utxoData = [
      81, 108, 50, 181, 0, 73, 91, 197, 221, 215, 106, 69, 5, 107, 146, 252, 37,
      252, 123, 175, 62, 200, 168, 230, 111, 6, 217, 71, 108, 186, 184, 83, 1,
      1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
      1, 1, 1, 1, 1, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    const deserializedUtxo: Utxo_IdlType =
      LightSystemProgram.program.coder.types.decode(
        'Utxo',
        Buffer.from(utxoData),
      );

    expect(deserializedUtxo.data).toBe(null);
    expect(deserializedUtxo.address).toBe(null);
    expect(deserializedUtxo.lamports.eq(bn(3))).toBe(true);
    expect(
      deserializedUtxo.owner.equals(
        new PublicKey('6UqiSPd2mRCTTwkzhcs1M6DGYsqHWd5jiPueX3LwDMXQ'),
      ),
    ).toBe(true);
    expect(
      JSON.stringify(deserializedUtxo.blinding) ===
        JSON.stringify(new Array(32).fill(1)),
    ).toBe(true);
  });

  it('serialize out utxo ', async () => {
    const utxoData = [
      65, 108, 61, 176, 52, 117, 234, 133, 198, 175, 67, 171, 12, 47, 143, 190,
      40, 85, 133, 139, 248, 63, 224, 103, 49, 223, 64, 138, 92, 25, 160, 29, 2,
      2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
      2, 2, 2, 2, 2, 2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    LightSystemProgram.program.coder.types.decode(
      'Compr',
      Buffer.from(utxoData),
    );
  });

  it('serialize event ', async () => {
    const data = [
      1, 0, 0, 0, 81, 108, 50, 181, 0, 73, 91, 197, 221, 215, 106, 69, 5, 107,
      146, 252, 37, 252, 123, 175, 62, 200, 168, 230, 111, 6, 217, 71, 108, 186,
      184, 83, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
      1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
      65, 108, 61, 176, 52, 117, 234, 133, 198, 175, 67, 171, 12, 47, 143, 190,
      40, 85, 133, 139, 248, 63, 224, 103, 49, 223, 64, 138, 92, 25, 160, 29, 2,
      2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
      2, 2, 2, 2, 2, 2, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0,
      0, 0, 0, 0, 0, 0,
    ];
    LightSystemProgram.program.coder.types.decode(
      'PublicTransactionEvent',
      Buffer.from(data),
    );
  });
});
