import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Forester Dashboard",
  description: "Light Protocol Forester monitoring dashboard",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="bg-gray-50 text-gray-900 antialiased">
        <main className="max-w-6xl mx-auto px-6 py-8">{children}</main>
      </body>
    </html>
  );
}
