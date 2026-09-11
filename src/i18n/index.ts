import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import {
  applyDocumentLocale,
  readStoredLocale,
  saveStoredLocale,
} from "../lib/locale";
import {
  DEFAULT_LOCALE,
  DEFAULT_NAMESPACE,
  I18N_NAMESPACES,
  type AppLocale,
} from "./config";
import { LOCALE_CHANGE_EVENT } from "./events";
import { resources } from "./resources";

const initialLocale = readStoredLocale();
applyDocumentLocale(initialLocale);

void i18n.use(initReactI18next).init({
  resources,
  lng: initialLocale,
  fallbackLng: DEFAULT_LOCALE,
  defaultNS: DEFAULT_NAMESPACE,
  ns: [...I18N_NAMESPACES],
  interpolation: {
    escapeValue: false,
  },
});

export async function changeLocale(locale: AppLocale): Promise<void> {
  await i18n.changeLanguage(locale);
  saveStoredLocale(locale);
  applyDocumentLocale(locale);
  window.dispatchEvent(new Event(LOCALE_CHANGE_EVENT));
}

export default i18n;
