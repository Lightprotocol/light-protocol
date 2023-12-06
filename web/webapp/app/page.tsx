"use client"
import React, { ReactNode, useEffect, useState } from "react";
import { AppShell, Burger, Group, Stack, Title } from "@mantine/core";
import { ADMIN_AUTH_KEYPAIR, useWallet } from "@lightprotocol/zk.js";
import { useUser } from "../state/hooks/useUser";
import { useConnection } from "@solana/wallet-adapter-react";
import { Transactions } from "../components/Transactions";
import { Assets } from "../components/Assets";
import { useDisclosure } from "@mantine/hooks";
import { ShieldSendModal } from "../components/Modal";
import { Navbar } from "../components/Navbar";
import { usePathname , useRouter} from "next/navigation";

export default function Page() {
  console.log("Page component rendered");

  const wallet = useWallet(
    ADMIN_AUTH_KEYPAIR,
    "https://api.devnet.solana.com",
    false
  );

  const { user, initUser, isLoading, error } = useUser();
  const { connection } = useConnection();

  const [balance, setBalance] = useState(0);
  // TODO: replace with login action.
  // wallet must be available state, but kept separate from login
  useEffect(() => {
    console.log("user", user?.account.getPublicKey());
    console.log("isLoading", isLoading);
    console.log("error", error);
    console.log("walletpubkey", wallet.publicKey.toBase58());
    if (wallet && !user && !isLoading && !error) {
      initUser({ connection, wallet });
    }
    (async () => {
      let balance = await connection.getBalance(wallet.publicKey);
      setBalance(balance);
    })();
  }, []);

  if (isLoading) {
    return <div>Logging in...</div>;
  }

  if (!user) {
    return <div>Please log in</div>;
  }

  return (
    <Shell>

      <Stack align="center">
        
        Public Balance: {balance}
        <Assets />
        <Transactions />
      </Stack>
    </Shell>
  );
}



function Shell({ children }: { children: ReactNode }){
  const [opened, { toggle }] = useDisclosure();
  const router = useRouter();
  const path = usePathname();
  return (
    <AppShell
      layout="alt"
      header={{ height: 60 }}
      navbar={{ width: 250, breakpoint: "sm", collapsed: { mobile: !opened } }}
      padding="md"
    >
      <Navbar router={router} path={path} />
      <AppShell.Header>
        <Group justify="space-between" h="100%" px="md" pl={"lg"}>
          <Burger opened={opened} onClick={toggle} hiddenFrom="sm" size="sm" />
          <Title size={"sm"}> My assets</Title>
          <Group>
            <ShieldSendModal />
            zk account wallet connection
          </Group>
        </Group>
      </AppShell.Header>
      <AppShell.Main bg={"#f3f6f9"}>
        {children}
      </AppShell.Main>
    </AppShell>
  )
}