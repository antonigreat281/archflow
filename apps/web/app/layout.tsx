import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Archflow",
  description: "Architecture diagrams as code",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" dir="ltr" suppressHydrationWarning>
      <body>{children}</body>
    </html>
  );
}
