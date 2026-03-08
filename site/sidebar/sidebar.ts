import type { Sidebar } from "vocs";

const docs = [
  {
    text: "Introduction",
    items: [
      { text: "Getting Started", link: "/introduction/getting-started" },
      { text: "Installation", link: "/introduction/installation" },
    ],
  },
  {
    text: "Core Concepts",
    items: [
      { text: "Accounts", link: "/core/accounts" },
      { text: "Output Modes", link: "/core/output" },
      { text: "Safety", link: "/core/safety" },
    ],
  },
  {
    text: "Hyperliquid",
    items: [
      { text: "Overview", link: "/protocols/hyperliquid" },
    ],
  },
  {
    text: "Polymarket",
    items: [
      { text: "Overview", link: "/protocols/polymarket" },
    ],
  },
  {
    text: "Reference",
    items: [
      { text: "Limitations", link: "/reference/limitations" },
      { text: "v0.1.0 Release Notes", link: "/reference/v0-1-0" },
    ],
  },
];

export const sidebar: Sidebar = {
  "/": [],
  "/introduction": docs,
  "/core": docs,
  "/protocols": docs,
  "/reference": docs,
};
