import { beforeEach, describe, expect, it } from "vitest";
import i18n, { changeLocale } from "./index";

describe("i18n", () => {
  beforeEach(async () => {
    localStorage.clear();
    await changeLocale("en-US");
  });

  it("translates common strings in English", () => {
    expect(i18n.t("common:action.export")).toBe("Export");
    expect(i18n.t("library:nav.library")).toBe("Library");
  });

  it("switches to Chinese", async () => {
    await changeLocale("zh-CN");
    expect(i18n.t("common:action.export")).toBe("导出");
    expect(i18n.t("library:nav.library")).toBe("图库");
  });

  it("falls back to English for missing keys", async () => {
    await changeLocale("zh-CN");
    expect(i18n.t("common:action.export", { lng: "en-US" })).toBe("Export");
  });
});
