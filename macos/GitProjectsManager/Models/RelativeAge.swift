import Foundation

/// How long ago `date` was, in leogit's vocabulary ("just now", "5 minutes
/// ago", "2 years ago"), and the instant that text next changes, so a label can
/// sleep until then instead of polling (FRONTEND.md §5.1).
///
/// Spelled out rather than `Date.RelativeFormatStyle`, whose named forms
/// ("yesterday", "last week") the Tauri app cannot match word for word.
struct RelativeAge {
    let text: String
    let nextChange: Date

    /// Smallest first. A 30-day month and a 365-day year: the label answers
    /// "roughly how long ago", and the exact time is in its tooltip.
    private static let units: [(name: String, seconds: TimeInterval)] = [
        ("minute", 60), ("hour", 3600), ("day", 86_400),
        ("month", 30 * 86_400), ("year", 365 * 86_400),
    ]

    init(of date: Date, now: Date) {
        // A date in the future (a clock set back) reads as "just now".
        let elapsed = max(now.timeIntervalSince(date), 0)
        guard let index = Self.units.lastIndex(where: { elapsed >= $0.seconds }) else {
            text = "just now"
            nextChange = date + Self.units[0].seconds
            return
        }
        let unit = Self.units[index]
        let count = (elapsed / unit.seconds).rounded(.down)
        text = "\(Int(count)) \(unit.name)\(count == 1 ? "" : "s") ago"
        // The next count of this unit, or the next unit if it comes first
        // (12 months ago turns into 1 year ago at 365 days, not 390).
        var untilNext = (count + 1) * unit.seconds
        if index + 1 < Self.units.count { untilNext = min(untilNext, Self.units[index + 1].seconds) }
        nextChange = date + untilNext
    }
}
