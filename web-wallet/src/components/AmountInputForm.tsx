import "../css/index.css";
import {
  majorScale,
  Pane,
  TextInput,
  Heading,
  Avatar,
  RefreshIcon,
} from "evergreen-ui";
import { Token } from "../constants";
import { useAtom } from "jotai";
import { activeTokenAtom } from "../state/activeAtoms";

export const AmountInputForm = ({
  amount,
  setAmount,
  checkAmount,
  ...rest
}: {
  amount: Number;
  setAmount: Function;
  checkAmount: Function;
}) => {
  const [activeToken, setActiveToken] = useAtom(activeTokenAtom);
  return (
    <Pane
      display="flex"
      marginTop="8px"
      flexDirection="row"
      justifyContent="space-between"
      marginBottom={majorScale(1)}
      alignItems="center"
    >
      <TextInput
        type="number"
        step="0.000000001"
        min="0"
        data-private
        height={majorScale(7)}
        backgroundColor="#f9fafc"
        width="67%"
        borderRadius="15px"
        textAlign="left"
        placeholder="Amount"
        className="amount-input-custom"
        // @ts-ignore
        value={amount}
        onChange={(e: any) => {
          setAmount(e.target.value);
        }}
        {...rest}
      ></TextInput>
      {activeToken === Token.SOL ? (
        <Pane>
          <Pane marginLeft="8px" display="flex" alignItems="center">
            <Avatar
              src="https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png"
              name="Solana Icon"
              size={32}
              marginRight="4px"
            />
            <Heading color="#474d66" size={700}>
              SOL
            </Heading>
          </Pane>
          {/* BELOW: FEATURE (USDC) */}
          <Pane display="flex" justifyContent="flex-end">
            <Pane
              cursor="pointer"
              onClick={() => {
                setActiveToken(Token.USDC);
              }}
              display="flex"
              justifyContent="center"
              alignItems="center"
              marginTop="4px"
              paddingX="8px"
              paddingY="4px"
              borderRadius="15px"
              backgroundColor="#f9fafc"
              maxWidth="70%"
            >
              <RefreshIcon size={10} />
              <Avatar
                src="https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png"
                name="USDC Icon"
                size={18}
                marginLeft="4px"
              />
            </Pane>
          </Pane>
        </Pane>
      ) : (
        <Pane>
          <Pane marginLeft="8px" display="flex" alignItems="center">
            <Avatar
              src="https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v/logo.png"
              name="USDC Icon"
              size={32}
              marginRight="4px"
            />
            <Heading color="#474d66" size={700}>
              USDC
            </Heading>
          </Pane>
          <Pane display="flex" justifyContent="flex-end">
            <Pane
              cursor="pointer"
              onClick={() => {
                setActiveToken(Token.SOL);
              }}
              display="flex"
              justifyContent="center"
              alignItems="center"
              marginTop="4px"
              paddingX="8px"
              paddingY="4px"
              borderRadius="15px"
              backgroundColor="#f9fafc"
              maxWidth="70%"
            >
              <RefreshIcon size={10} />
              <Avatar
                src="https://raw.githubusercontent.com/solana-labs/token-list/main/assets/mainnet/So11111111111111111111111111111111111111112/logo.png"
                name="Solana Icon"
                size={18}
                marginLeft="4px"
              />
            </Pane>
          </Pane>
        </Pane>
      )}
    </Pane>
  );
};
