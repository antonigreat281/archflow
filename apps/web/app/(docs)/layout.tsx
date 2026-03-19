import { Footer, Layout, Navbar } from "nextra-theme-docs";
import { Head } from "nextra/components";
import { getPageMap } from "nextra/page-map";
import "nextra-theme-docs/style.css";

const navbar = (
  <Navbar
    logo={<strong>Archflow</strong>}
    projectLink="https://github.com/soulee-dev/archflow"
  />
);

const footer = <Footer>MIT {new Date().getFullYear()} © Archflow</Footer>;

export default async function DocsLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const pageMap = await getPageMap();
  return (
    <>
      <Head />
      <Layout navbar={navbar} footer={footer} pageMap={pageMap}>
        {children}
      </Layout>
    </>
  );
}
