export const THEME_IDS = [
  "slate",
  "pulse",
  "neon",
  "vault",
  "atlas",
  "sakura",
  "moss",
  "solar",
  "ink",
  "coral",
  "oxide",
  "arctic",
  "prism",
] as const;

export type ThemeId = (typeof THEME_IDS)[number];

export const DEFAULT_THEME_ID: ThemeId = "slate";
