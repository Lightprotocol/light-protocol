import { Pane, Text, Link, majorScale } from "evergreen-ui";
export const Footer = () => (
  <>
    {" "}
    <Pane
      display="flex"
      justifyContent="center"
      marginTop={majorScale(2)}
      marginBottom={majorScale(1)}
    >
      <Text size={300} marginRight="8px">
        v1.5.1 (Mainnet-Beta)
      </Text>
      <Link
        size={300}
        marginRight="0px"
        color="neutral"
        target="_blank"
        href="https://github.com/Lightprotocol/light-protocol-program/tree/main/Audit"
      >
        Security
      </Link>
      <Link
        size={300}
        marginLeft="8px"
        color="neutral"
        target="_blank"
        href="https://twitter.com/LightProtocol"
      >
        {" "}
        Twitter{" "}
      </Link>
      <Link
        size={300}
        color="neutral"
        marginLeft="8px"
        target="_blank"
        href="https://discord.gg/WDAAaX6je2"
      >
        {" "}
        Discord{" "}
      </Link>
      <Link
        size={300}
        marginLeft="8px"
        color="neutral"
        target="_blank"
        href="https://docs.lightprotocol.com/"
      >
        {" "}
        Docs{" "}
      </Link>
      <Link
        size={300}
        marginLeft="8px"
        marginRight="2px"
        color="neutral"
        target="_blank"
        href="https://docs.lightprotocol.com/terms-of-use"
      >
        ToS
      </Link>{" "}
    </Pane>
    <Pane marginTop="8px" display="flex">
      <Text size={300} color="lightgrey">
        2023 ©️ Light Protocol Labs |
      </Text>
      <Link
        size={300}
        marginLeft="4px"
        color="neutral"
        target="_blank"
        href="https://api.coingecko.com/api/v3/simple/price?ids=solana&vs_currencies=usd"
      >
        <Text size={300} color="lightgrey">
          Rates provided by CoinGecko
        </Text>
      </Link>
    </Pane>
  </>
);
