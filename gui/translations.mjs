import en from "./locales/en.mjs";
import fr from "./locales/fr.mjs";
import es from "./locales/es.mjs";
import de from "./locales/de.mjs";
import ru from "./locales/ru.mjs";
import zh from "./locales/zh.mjs";
import ko from "./locales/ko.mjs";
import ja from "./locales/ja.mjs";
import id from "./locales/id.mjs";
import it from "./locales/it.mjs";
import pt from "./locales/pt.mjs";
import pl from "./locales/pl.mjs";
import tr from "./locales/tr.mjs";

export const translations = { en, fr, es, de, ru, zh, ko, ja, id, it, pt, pl, tr };

export const regionalTranslations = {
  "en-US": {
    "personalizationTitle": "Customize Relay your way.",
    "accentColor": "Accent color",
    "previewCopy": "Readable text with your chosen accent color.",
    "privacyCategoryPhone": "Phone numbers",
    "privacyCategoryAddress": "ZIP and postal addresses",
    "customActionClearMessages": "Clear messages",
    "customAllowedChannels": "Allowed command channel IDs"
  },
  "en-GB": {
    "personalizationTitle": "Personalise Relay your way.",
    "accentColor": "Accent colour",
    "previewCopy": "Readable text with your chosen accent colour.",
    "privacyCategoryPhone": "Telephone numbers",
    "privacyCategoryAddress": "Postal addresses",
    "customActionClearMessages": "Delete messages",
    "customAllowedChannels": "Allowed command channel IDs"
  },
  "en-IN": {
    "personalizationTitle": "Personalise Relay for your workspace.",
    "accentColor": "Accent colour",
    "previewCopy": "Readable text with your chosen accent colour.",
    "privacyCategoryPhone": "Mobile and telephone numbers",
    "privacyCategoryAddress": "Postal addresses and PIN codes",
    "customActionClearMessages": "Delete messages",
    "customAllowedChannels": "Permitted command channel IDs"
  }
};
