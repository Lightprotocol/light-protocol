"use client";
import { theme } from "../styles/theme";
import { MantineProvider } from "@mantine/core";

export function Providers({ children }: { children: React.ReactNode }) {
  return (
    <>
      <MantineProvider theme={theme}>{children}</MantineProvider>
    </>
  );
}
