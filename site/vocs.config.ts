import { defineConfig } from "vocs";
import { sidebar } from "./sidebar/sidebar";

export default defineConfig({
  title: "Heat — One CLI for All of Finance",
  description: "One CLI for all of finance — built for humans and AI agents",
  rootDir: ".",
  sidebar,
  theme: {
    colorScheme: "dark",
  },
  accentColor: "#FF6B35",
  logoUrl: "/logo.svg",
  iconUrl: "/logo.svg",
  font: {
    google: "Space Grotesk",
  },
  socials: [
    {
      link: "https://github.com/dzmbs/heat-cli",
      icon: "github",
    },
  ],
  topNav: [
    { link: "/introduction/getting-started", text: "Docs" },
    { link: "/protocols/hyperliquid", text: "Protocols" },
    { link: "/reference/limitations", text: "Reference" },
    { link: "/introduction/installation", text: "Install" },
  ],
});
