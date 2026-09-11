import {
  APP_LOCALES,
  DEFAULT_LOCALE,
  LOCALE_LABELS,
  LOCALE_STORAGE_KEY,
  type AppLocale,
} from "../i18n/config";

export function isAppLocale(
  value: string | null | undefined,
): value is AppLocale {
  return (
    value !== null &&
    value !== undefined &&
    APP_LOCALES.includes(value as AppLocale)
  );
}

export function resolveSystemLocale(): AppLocale {
  if (typeof navigator === "undefined") {
    return DEFAULT_LOCALE;
  }
  const language = navigator.language.toLowerCase();
  if (language.startsWith("zh")) {
    return "zh-CN";
  }
  return "en-US";
}

export function readStoredLocale(): AppLocale {
  try {
    const stored = localStorage.getItem(LOCALE_STORAGE_KEY);
    if (isAppLocale(stored)) {
      return stored;
    }
  } catch {
    return resolveSystemLocale();
  }
  return resolveSystemLocale();
}

export function saveStoredLocale(locale: AppLocale): void {
  try {
    localStorage.setItem(LOCALE_STORAGE_KEY, locale);
  } catch {
    // ignore storage failures
  }
}

export function localeLabel(locale: AppLocale): string {
  return LOCALE_LABELS[locale];
}

export function applyDocumentLocale(locale: AppLocale): void {
  document.documentElement.lang = locale;
}
