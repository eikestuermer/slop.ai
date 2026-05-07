// Slop AI uses i18next with the ICU plugin for production-grade
// internationalization. ICU MessageFormat 2 handles plurals, gender,
// nesting, and select expressions out of the box. The same translation
// files work in the desktop app and the web companion.

import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import ICU from "i18next-icu";
import HttpBackend from "i18next-http-backend";
import LanguageDetector from "i18next-browser-languagedetector";

import en from "./locales/en/common.json";
import de from "./locales/de/common.json";
import es from "./locales/es/common.json";
import fr from "./locales/fr/common.json";
import ja from "./locales/ja/common.json";

void i18n
  .use(ICU)
  .use(HttpBackend)
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: { common: en },
      de: { common: de },
      es: { common: es },
      fr: { common: fr },
      ja: { common: ja },
    },
    fallbackLng: "en",
    defaultNS: "common",
    supportedLngs: ["en", "de", "es", "fr", "ja"],
    interpolation: {
      // i18next-icu overrides interpolation; this is just a guardrail.
      escapeValue: false,
    },
    detection: {
      order: ["localStorage", "navigator"],
      caches: ["localStorage"],
    },
  });

export default i18n;
