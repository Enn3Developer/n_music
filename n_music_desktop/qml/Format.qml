pragma Singleton
import QtQuick
import NMusic

// Numbers and durations as the interface shows them.
QtObject {
    function number(value: int): string {
        return Number(value).toLocaleString(Qt.locale(), "f", 0);
    }

    /// `value` in `one` or `many`, whose `%1` it replaces.
    function count(value: int, one: string, many: string): string {
        return (value === 1 ? one : many).arg(number(value));
    }

    /// A total length like `21 h 40 min`.
    function duration(seconds: real): string {
        const minutes = Math.round(seconds / 60);
        const hours = Math.floor(minutes / 60);
        if (hours > 0)
            return Tr.t.duration_hours.arg(number(hours)).arg(minutes % 60);
        return Tr.t.duration_minutes.arg(minutes);
    }
}
