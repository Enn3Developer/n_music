//! Qt base classes of the bridges' objects, declared once: each declaration generates casting
//! functions, which would clash between bridges.

#[cxx_qt::bridge]
pub mod qobject {
    extern "C++Qt" {
        include!(<QtCore/QAbstractListModel>);
        #[qobject]
        type QAbstractListModel;
    }
}
