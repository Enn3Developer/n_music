//! The interface strings, in the language picked in the settings or the system's.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// Every string by key, as a map that QML reads as an object.
        #[qproperty(QVariant, strings)]
        /// "System default", then the bundled languages.
        #[qproperty(QStringList, languages)]
        /// The chosen entry of `languages`; saved when changed.
        #[qproperty(i32, language)]
        type Translations = super::TranslationsRust;
    }

    impl cxx_qt::Initialize for Translations {}
}

use crate::{i18n, settings};
use core::pin::Pin;
use cxx_qt_lib::{QList, QMap, QMapPair_QString_QVariant, QString, QStringList, QVariant};

#[derive(Default)]
pub struct TranslationsRust {
    strings: QVariant,
    languages: QStringList,
    language: i32,
}

impl cxx_qt::Initialize for qobject::Translations {
    fn initialize(mut self: Pin<&mut Self>) {
        let saved = settings::ui().get().locale.clone();
        let languages = i18n::languages();
        let index = saved
            .as_deref()
            .and_then(|saved| languages.iter().position(|&(code, _)| code == saved))
            .map_or(0, |index| index as i32 + 1);
        self.as_mut().load(saved.as_deref());
        self.as_mut().set_language(index);
        self.as_mut()
            .on_language_changed(|mut translations| {
                let code = usize::try_from(*translations.language() - 1)
                    .ok()
                    .and_then(|index| i18n::languages().get(index).map(|&(code, _)| code));
                settings::ui().update(|ui| ui.locale = code.map(String::from));
                translations.as_mut().load(code);
            })
            .release();
    }
}

impl qobject::Translations {
    /// Shows `saved`, or the system's language when `None`.
    fn load(mut self: Pin<&mut Self>, saved: Option<&str>) {
        let strings = i18n::strings(i18n::resolve(saved));
        let system = i18n::resolve(None);
        let system_name = i18n::languages()
            .into_iter()
            .find(|&(code, _)| code == system)
            .map_or("English", |(_, name)| name);
        let mut names = QList::<QString>::default();
        let label = strings
            .get("language_system")
            .map_or("System default", String::as_str);
        names.append(QString::from(&format!("{label} ({system_name})")));
        for (_, name) in i18n::languages() {
            names.append(QString::from(name));
        }
        let mut map = QMap::<QMapPair_QString_QVariant>::default();
        for (key, value) in &strings {
            map.insert(QString::from(key), QVariant::from(&QString::from(value)));
        }
        self.as_mut().set_languages(QStringList::from(&names));
        self.set_strings(QVariant::from(&map));
    }
}
