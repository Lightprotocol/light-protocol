"use client"
import React from 'react';
import { AppShell, Burger, Group, Title } from "@mantine/core";
import { useDisclosure } from "@mantine/hooks";
import { Navbar } from '../Navbar';
import { ShieldSendModal } from "../Modal";
import { usePathname, useRouter } from "next/navigation";
import { LayoutProps } from '.next/types/app/page';

export const Layout: React.FC = ({ children }: LayoutProps) => {
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
  );
};