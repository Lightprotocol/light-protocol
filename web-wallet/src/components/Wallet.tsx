import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import "../css/wallet.css";

// Default styles that can be overridden by your app
require("@solana/wallet-adapter-react-ui/styles.css");

export const Wallet = () => {
  return <WalletMultiButton data-private className="light-wallet-button" />;
};
