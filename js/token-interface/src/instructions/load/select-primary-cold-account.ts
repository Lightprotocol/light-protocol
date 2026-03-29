import { ParsedTokenAccount, bn } from "@lightprotocol/stateless.js";
import { Buffer } from "buffer";
import { COLD_SOURCE_TYPES, TokenAccountSource } from "../../read/get-account";

/**
 * Default load policy: select one deterministic cold compressed account.
 * Priority is highest amount, then highest leaf index.
 */
export function selectPrimaryColdCompressedAccountForLoad(
  sources: TokenAccountSource[],
): ParsedTokenAccount | null {
  const candidates: ParsedTokenAccount[] = [];
  for (const source of sources) {
    if (!COLD_SOURCE_TYPES.has(source.type) || !source.loadContext) {
      continue;
    }
    const loadContext = source.loadContext;
    const fullData = source.accountInfo.data;
    const discriminatorBytes = fullData.subarray(
      0,
      Math.min(8, fullData.length),
    );
    const accountDataBytes =
      fullData.length > 8 ? fullData.subarray(8) : Buffer.alloc(0);

    const compressedAccount = {
      treeInfo: loadContext.treeInfo,
      hash: loadContext.hash,
      leafIndex: loadContext.leafIndex,
      proveByIndex: loadContext.proveByIndex,
      owner: source.accountInfo.owner,
      lamports: bn(source.accountInfo.lamports),
      address: null,
      data:
        fullData.length === 0
          ? null
          : {
              discriminator: Array.from(discriminatorBytes),
              data: Buffer.from(accountDataBytes),
              dataHash: new Array(32).fill(0),
            },
      readOnly: false,
    } satisfies ParsedTokenAccount["compressedAccount"];

    const state = !source.parsed.isInitialized
      ? 0
      : source.parsed.isFrozen
        ? 2
        : 1;

    candidates.push({
      compressedAccount,
      parsed: {
        mint: source.parsed.mint,
        owner: source.parsed.owner,
        amount: bn(source.parsed.amount.toString()),
        delegate: source.parsed.delegate,
        state,
        tlv: source.parsed.tlvData.length > 0 ? source.parsed.tlvData : null,
      },
    });
  }

  if (candidates.length === 0) {
    return null;
  }

  candidates.sort((a, b) => {
    const amountA = BigInt(a.parsed.amount.toString());
    const amountB = BigInt(b.parsed.amount.toString());
    if (amountB > amountA) return 1;
    if (amountB < amountA) return -1;
    return b.compressedAccount.leafIndex - a.compressedAccount.leafIndex;
  });

  return candidates[0];
}
