import { useWallet } from "@solana/wallet-adapter-react";
import { useCallback } from "react";
import { sign } from "tweetnacl";
import { Button } from "evergreen-ui";
import { useAtom } from "jotai";
import { bufferAtom } from "../state/utilAtoms";
import { derivedUserKeypairsAtom } from "../state/userAtoms";

export const SignMessageButton = ({
  message,
  label,
}: {
  message: string;
  label: string;
}) => {
  const { publicKey, signMessage } = useWallet();
  const [_, setShowBuffer] = useAtom(bufferAtom);
  const [__, deriveKeypairs] = useAtom(derivedUserKeypairsAtom);
  const onClick = useCallback(async () => {
    try {
      setShowBuffer(true);
      if (!publicKey) throw new Error("Wallet not connected!");
      if (!signMessage)
        throw new Error("Wallet does not support message signing!");
      const encodedMessage = new TextEncoder().encode(message);
      const signature = await signMessage(encodedMessage);
      if (!sign.detached.verify(encodedMessage, signature, publicKey.toBytes()))
        throw new Error("Invalid signature!");
      deriveKeypairs(signature);
      //TODO: is not registered ? -> switch to "storeAccount" step.
    } catch (error: any) {
      error.message = `Signing failed: ${error?.message}`;
      setShowBuffer(false);
    }
  }, [publicKey, signMessage]);

  return signMessage ? (
    <Button
      onClick={onClick}
      disabled={!publicKey}
      appearance="primary"
      size="large"
      width="-webkit-fill-available"
    >
      {label}
    </Button>
  ) : (
    <Button disabled size="large" width="-webkit-fill-available">
      Your wallet does not support message signing
    </Button>
  );
};
