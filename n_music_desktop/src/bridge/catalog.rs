//! What the library holds, for suggestions while writing filter rules.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qstringlist.h");
        type QStringList = cxx_qt_lib::QStringList;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
    }

    #[auto_cxx_name]
    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        type Catalog = super::CatalogRust;

        /// The library's values of `field` (`artist`, `album_artist`, `album`, `genre` or
        /// `codec`), each once whatever its case, sorted.
        #[qinvokable]
        fn values(self: &Catalog, field: &QString) -> QStringList;

        /// The path of a picked folder as the library writes locations; empty unless local.
        #[qinvokable]
        fn folder(self: &Catalog, url: &QUrl) -> QString;
    }
}

use crate::hub::hub;
use cxx_qt_lib::{QList, QString, QStringList, QUrl};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct CatalogRust;

impl qobject::Catalog {
    fn values(&self, field: &QString) -> QStringList {
        let catalog = hub().library().read();
        // Keyed by lower case, so `J-Pop` and `j-pop` show once.
        let mut values = BTreeMap::<String, String>::new();
        let mut add = |value: &str| {
            let value = value.trim();
            if !value.is_empty() {
                values
                    .entry(value.to_lowercase())
                    .or_insert_with(|| value.to_string());
            }
        };
        let field = field.to_string();
        for track in catalog.tracks() {
            match field.as_str() {
                "artist" => track.artists.iter().for_each(|artist| add(artist)),
                "album_artist" => track.album_artist.iter().for_each(|artist| add(artist)),
                "album" => track.album.iter().for_each(|album| add(album)),
                "genre" => track.genres.iter().for_each(|genre| add(genre)),
                "codec" => track.codec.iter().for_each(|codec| add(codec)),
                _ => return QStringList::default(),
            }
        }
        let mut list = QList::<QString>::default();
        for value in values.into_values() {
            list.append(QString::from(&value));
        }
        QStringList::from(&list)
    }

    fn folder(&self, url: &QUrl) -> QString {
        let Some(path) = url.to_local_file() else {
            return QString::default();
        };
        // Qt writes `/` everywhere; the scan writes the platform's separators.
        let path: PathBuf = Path::new(&path.to_string()).components().collect();
        QString::from(&*path.to_string_lossy())
    }
}
