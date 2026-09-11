import { useCallback, useSyncExternalStore } from "react";
import i18n, { changeLocale } from "../i18n";
import { LOCALE_CHANGE_EVENT } from "../i18n/events";
import {
  APP_LOCALES,
  type AppLocale,
} from "../i18n/config";
import { isAppLocale, localeLabel, readStoredLocale } from "../lib/locale";

function subscribe(onStoreChange: () => void) {
  const handler = () => onStoreChange();
  window.addEventListener(LOCALE_CHANGE_EVENT, handler);
  i18n.on("languageChanged", handler);
  return () => {
    window.removeEventListener(LOCALE_CHANGE_EVENT, handler);
    i18n.off("languageChanged", handler);
  };
}

function getLocaleSnapshot(): AppLocale {
  const language = i18n.language;
  if (isAppLocale(language)) {
    return language;
  }
  return readStoredLocale();
}

export function useAppLocale() {
  const locale = useSyncExternalStore(subscribe, getLocaleSnapshot, getLocaleSnapshot);

  const setLocale = useCallback((next: AppLocale) => {
    void changeLocale(next);
  }, []);

  return {
    locale,
    setLocale,
    locales: APP_LOCALES,
    localeLabel,
  };
}
