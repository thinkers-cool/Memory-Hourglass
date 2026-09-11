import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { DEFAULT_THEME_ID, THEME_IDS } from "./themes.registry";

const root = resolve(import.meta.dirname, "../..");

describe("theme registry", () => {
  it("lists every theme in index.css daisyui plugin config", () => {
    const indexCss = readFileSync(resolve(root, "src/index.css"), "utf8");
    const pluginMatch = indexCss.match(/@plugin "daisyui" \{([^}]+)\}/);
    expect(pluginMatch).not.toBeNull();
    const pluginBlock = pluginMatch![1];
    for (const themeId of THEME_IDS) {
      expect(pluginBlock).toContain(themeId);
    }
  });

  it("defines every theme in memhg.css", () => {
    const memhgCss = readFileSync(resolve(root, "src/styles/themes/memhg.css"), "utf8");
    for (const themeId of THEME_IDS) {
      expect(memhgCss).toContain(`name: "${themeId}"`);
    }
  });

  it("marks default theme only in index.css plugin config", () => {
    const indexCss = readFileSync(resolve(root, "src/index.css"), "utf8");
    const memhgCss = readFileSync(resolve(root, "src/styles/themes/memhg.css"), "utf8");
    expect(indexCss).toContain(`${DEFAULT_THEME_ID} --default`);
    expect(memhgCss).not.toContain("default: true");
  });
});
