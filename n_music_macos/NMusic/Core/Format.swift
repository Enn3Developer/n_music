import Foundation

/// Numbers, times and sizes as the interface shows them: exactly the Qt app's `Format.qml`
/// (spec §14) plus the track-length clock of `src/format.rs` (`Format.length`).
@MainActor
enum Format {
    private static let integer: NumberFormatter = {
        let formatter = NumberFormatter()
        formatter.locale = .current
        formatter.numberStyle = .decimal
        formatter.maximumFractionDigits = 0
        return formatter
    }()

    private static func decimal(_ value: Double, digits: Int) -> String {
        let formatter = NumberFormatter()
        formatter.locale = .current
        formatter.numberStyle = .decimal
        formatter.minimumFractionDigits = digits
        formatter.maximumFractionDigits = digits
        return formatter.string(from: NSNumber(value: value)) ?? String(value)
    }

    /// A locale-grouped integer: `1,234`.
    static func number<T: BinaryInteger>(_ value: T) -> String {
        integer.string(from: NSNumber(value: Int64(clamping: value))) ?? String(value)
    }

    /// `value` in the string `one` when it is 1, else `many`, whose `%1` it fills: e.g.
    /// `Format.count(n, "track_one", "tracks_many")`. English-style plurals only, as Qt.
    static func count<T: BinaryInteger>(_ value: T, _ oneKey: String, _ manyKey: String) -> String {
        tr(value == 1 ? oneKey : manyKey, number(value))
    }

    /// A position like `4:31`, or `1:02:09` from an hour; floored, negative is `0:00`. For the
    /// player's clocks.
    nonisolated static func clock(_ seconds: Double) -> String {
        let total = seconds.isFinite && seconds > 0 ? Int(seconds.rounded(.down)) : 0
        let hours = total / 3600
        let minutes = total / 60 % 60
        let rest = total % 60
        if hours > 0 {
            return "\(hours):\(minutes < 10 ? "0" : "")\(minutes):\(rest < 10 ? "0" : "")\(rest)"
        }
        return "\(minutes):\(rest < 10 ? "0" : "")\(rest)"
    }

    /// A track length like `4:31` (rounded, as the track lists show it); empty when unknown.
    nonisolated static func length(_ seconds: Double) -> String {
        guard seconds.isFinite, seconds > 0 else { return "" }
        let total = Int(seconds.rounded())
        let hours = total / 3600
        let minutes = total / 60 % 60
        let rest = total % 60
        if hours > 0 {
            return "\(hours):\(minutes < 10 ? "0" : "")\(minutes):\(rest < 10 ? "0" : "")\(rest)"
        }
        return "\(minutes):\(rest < 10 ? "0" : "")\(rest)"
    }

    /// A size on disk like `512 MB` or `1.5 GB`, in units of 1024: whole megabytes below
    /// 1024 MiB; gigabytes with one decimal only below 10 GB and only when it isn't `.0`.
    static func size(_ bytes: Double) -> String {
        let megabytes = bytes / 1_048_576
        if megabytes < 1024 {
            return tr("size_mb", decimal(megabytes.rounded(), digits: 0))
        }
        let gigabytes = megabytes / 1024
        let tenths = Int((gigabytes * 10).rounded())
        let digits = gigabytes < 10 && tenths % 10 != 0 ? 1 : 0
        return tr("size_gb", decimal(gigabytes, digits: digits))
    }

    /// Whole calendar days from the day of Unix time `seconds` to today (DST-safe).
    static func daysSince(_ seconds: Double) -> Int {
        let calendar = Calendar.current
        let then = calendar.startOfDay(for: Date(timeIntervalSince1970: seconds))
        let now = calendar.startOfDay(for: Date())
        return Int((now.timeIntervalSince(then) / 86_400).rounded())
    }

    private static let agoDate: DateFormatter = {
        let formatter = DateFormatter()
        formatter.locale = .current
        formatter.dateFormat = "d MMM yyyy"
        return formatter
    }()

    /// When Unix time `seconds` was: `Today`, `Yesterday`, `3 days ago`, `Last week`,
    /// `2 weeks ago`, then from 35 days a date like `4 Mar 2025`.
    static func ago(_ seconds: Double) -> String {
        let days = daysSince(seconds)
        if days <= 0 { return tr("ago_today") }
        if days == 1 { return tr("ago_yesterday") }
        if days < 7 { return tr("ago_days", days) }
        if days < 14 { return tr("ago_last_week") }
        if days < 35 { return tr("ago_weeks", days / 7) }
        return agoDate.string(from: Date(timeIntervalSince1970: seconds))
    }

    /// Codec, sample rate and bit depth, like `FLAC · 44.1 kHz · 16-bit`; empty parts dropped.
    /// `codec` is shown as given: pass it in capitals.
    static func audio(codec: String?, sampleRate: UInt32?, bits: UInt32?) -> String {
        var parts: [String] = []
        if let codec, !codec.isEmpty { parts.append(codec) }
        if let rate = sampleRate, rate > 0 {
            let khz = rate % 1000 == 0 ? String(rate / 1000) : decimal(Double(rate) / 1000, digits: 1)
            parts.append(tr("sample_rate_khz", khz))
        }
        if let bits, bits > 0 { parts.append(tr("bit_depth", bits)) }
        return parts.joined(separator: " · ")
    }

    /// The ReplayGain playback can apply, like `ReplayGain album −6.2 dB`: the album's, else
    /// the track's; empty without either. The sign is always shown, with a real minus.
    static func gain(album: Double?, track: Double?) -> String {
        let useAlbum = album.map { !$0.isNaN } ?? false
        guard let value = useAlbum ? album : track, !value.isNaN else { return "" }
        let decibels = (value < 0 ? "−" : "+") + decimal(abs(value), digits: 1)
        return tr(useAlbum ? "replaygain_album" : "replaygain_track", decibels)
    }

    /// A total length like `21 h 40 min`, or `13 days 2 h` from a day; rounded to minutes.
    static func duration(_ seconds: Double) -> String {
        let minutes = Int((seconds / 60).rounded())
        let hours = minutes / 60
        if hours >= 24 {
            let days = hours / 24
            return tr(days == 1 ? "duration_day" : "duration_days", number(days), hours % 24)
        }
        if hours > 0 {
            return tr("duration_hours", number(hours), minutes % 60)
        }
        return tr("duration_minutes", minutes)
    }
}
