import { useCallback } from "react";
import { useAction } from "../../state/hooks/useAction";
import { PublicKey } from "@solana/web3.js";
import { SendFormValues } from "../../components/Form";

export function useSend() {
  const { transfer, decompress } = useAction();

  const send = useCallback(
    async (values: SendFormValues, isUnshield: boolean) => {
      try {
        if (isUnshield) {
          console.log("unshielding");
          await decompress({
            token: values.token,
            recipient: new PublicKey(values.recipient),
            publicAmountSol: values.token === "SOL" ? values.amount : undefined,
            publicAmountSpl: values.token !== "SOL" ? values.amount : undefined,
          });
        } else {
          console.log("transferring");
          await transfer({
            token: values.token,
            recipient: values.recipient,
            amountSol: values.token === "SOL" ? values.amount : undefined,
            amountSpl: values.token === "SOL" ? undefined : values.amount,
          });
        }
      } catch (e) {
        console.error(e);
        throw e;
      }
    },
    [decompress, transfer]
  );

  return send;
}
