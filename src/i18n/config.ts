export const LOCALE_STORAGE_KEY = "memhg-locale";

export const APP_LOCALES = ["en-US", "zh-CN"] as const;

export type AppLocale = (typeof APP_LOCALES)[number];

export const DEFAULT_LOCALE: AppLocale = "en-US";

export const LOCALE_LABELS: Record<AppLocale, string> = {
  "en-US": "English",
  "zh-CN": "简体中文",
};

export const I18N_NAMESPACES = ["common", "library", "dialogs", "errors"] as const;

export type I18nNamespace = (typeof I18N_NAMESPACES)[number];

export const DEFAULT_NAMESPACE: I18nNamespace = "common";
