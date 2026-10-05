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

    /// A position or length like `4:31`, or `1:02:09` from an hour.
    function clock(seconds: real): string {
        const total = seconds > 0 ? Math.floor(seconds) : 0;
        const hours = Math.floor(total / 3600);
        const minutes = Math.floor(total / 60) % 60;
        const rest = (total % 60 < 10 ? "0" : "") + total % 60;
        if (hours > 0)
            return hours + ":" + (minutes < 10 ? "0" : "") + minutes + ":" + rest;
        return minutes + ":" + rest;
    }

    /// Whole days from the day of Unix time `seconds` to today.
    function daysSince(seconds: real): int {
        const then = new Date(seconds * 1000);
        const now = new Date();
        const start = date => new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
        // Rounded: a day across a daylight saving change is not 24 hours long.
        return Math.round((start(now) - start(then)) / 86400000);
    }

    /// When Unix time `seconds` was: `Today`, `3 days ago`, `Last week`… then a date.
    function ago(seconds: real): string {
        const days = daysSince(seconds);
        if (days <= 0)
            return Tr.t.ago_today;
        if (days === 1)
            return Tr.t.ago_yesterday;
        if (days < 7)
            return Tr.t.ago_days.arg(days);
        if (days < 14)
            return Tr.t.ago_last_week;
        if (days < 35)
            return Tr.t.ago_weeks.arg(Math.floor(days / 7));
        return new Date(seconds * 1000).toLocaleDateString(Qt.locale(), "d MMM yyyy");
    }

    /// Codec, sample rate and bit depth, like `FLAC · 44.1 kHz · 16-bit`.
    function audio(codec: string, sampleRate: int, bits: int): string {
        const parts = [];
        if (codec !== "")
            parts.push(codec);
        if (sampleRate > 0) {
            const rate = sampleRate % 1000 === 0 ? sampleRate / 1000 : (sampleRate / 1000).toLocaleString(Qt.locale(), "f", 1);
            parts.push(Tr.t.sample_rate_khz.arg(rate));
        }
        if (bits > 0)
            parts.push(Tr.t.bit_depth.arg(bits));
        return parts.join(" · ");
    }

    /// The ReplayGain playback can apply, like `ReplayGain album −6.2 dB`: the album's, else
    /// the track's; empty without either.
    function gain(albumGain: real, trackGain: real): string {
        const album = !isNaN(albumGain);
        const value = album ? albumGain : trackGain;
        if (isNaN(value))
            return "";
        const decibels = (value < 0 ? "−" : "+") + Math.abs(value).toLocaleString(Qt.locale(), "f", 1);
        return (album ? Tr.t.replaygain_album : Tr.t.replaygain_track).arg(decibels);
    }

    /// A total length like `21 h 40 min`, or `13 days 2 h` from a day.
    function duration(seconds: real): string {
        const minutes = Math.round(seconds / 60);
        const hours = Math.floor(minutes / 60);
        if (hours >= 24) {
            const days = Math.floor(hours / 24);
            return (days === 1 ? Tr.t.duration_day : Tr.t.duration_days).arg(number(days)).arg(hours % 24);
        }
        if (hours > 0)
            return Tr.t.duration_hours.arg(number(hours)).arg(minutes % 60);
        return Tr.t.duration_minutes.arg(minutes);
    }
}
