// Qi Sanction & WidgetKit Extensions (Lock Screen + Home Screen 2x2 / 4x2)
// Shared App Group: group.eu.org.cluas.qi
// Note: No @main here so this file compiles safely inside hosts/ios/Sources alongside bridge files.

import Foundation

#if canImport(WidgetKit) && canImport(SwiftUI)
import SwiftUI
import WidgetKit

public struct QiWidgetSnapshot: Codable, Sendable {
    public let score: Int
    public let state: String
    public let stateBadge: String
    public let poeticLine: String
    public let metricsLine: String
    public let solarTerm: String
    public let privacyMode: String // "metaphor" | "metrics"
    public let locale: String      // "zh-CN" | "en"
}

public struct QiWidgetEntry: TimelineEntry, Sendable {
    public let date: Date
    public let snapshot: QiWidgetSnapshot

    public var isChinese: Bool {
        snapshot.locale.lowercased().hasPrefix("zh")
    }

    public var brandTitle: String {
        isChinese ? "知气" : "Qi"
    }

    public var isPoeticPrivacy: Bool {
        snapshot.privacyMode != "metrics"
    }
}

public struct QiTimelineProvider: TimelineProvider {
    public static let appGroupSuite = "group.eu.org.cluas.qi"
    public static let snapshotKey = "qi_widget_snapshot"

    public init() {}

    public func placeholder(in context: Context) -> QiWidgetEntry {
        QiWidgetEntry(date: Date(), snapshot: Self.defaultSnapshot())
    }

    public func getSnapshot(in context: Context, completion: @escaping (QiWidgetEntry) -> Void) {
        completion(QiWidgetEntry(date: Date(), snapshot: Self.loadSnapshot()))
    }

    public func getTimeline(in context: Context, completion: @escaping (Timeline<QiWidgetEntry>) -> Void) {
        let entry = QiWidgetEntry(date: Date(), snapshot: Self.loadSnapshot())
        let nextRefresh = Calendar.current.date(byAdding: .minute, value: 30, to: entry.date)
            ?? entry.date.addingTimeInterval(1800)
        completion(Timeline(entries: [entry], policy: .after(nextRefresh)))
    }

    public static func loadSnapshot() -> QiWidgetSnapshot {
        guard
            let defaults = UserDefaults(suiteName: appGroupSuite),
            let raw = defaults.string(forKey: snapshotKey),
            let data = raw.data(using: .utf8),
            let decoded = try? JSONDecoder().decode(QiWidgetSnapshot.self, from: data)
        else {
            return defaultSnapshot()
        }
        return decoded
    }

    public static func defaultSnapshot() -> QiWidgetSnapshot {
        QiWidgetSnapshot(
            score: 84,
            state: "prime_ready",
            stateBadge: "温润充沛",
            poeticLine: "晨露未晞，今日宜轻行慢饮。",
            metricsLine: "HRV 稳步回升 · 深睡 2.1 h",
            solarTerm: "立春",
            privacyMode: "metaphor",
            locale: "zh-CN"
        )
    }
}

public struct QiSanctuaryWidgetView: View {
    @Environment(\.widgetFamily) private var family
    public let entry: QiWidgetEntry

    public init(entry: QiWidgetEntry) {
        self.entry = entry
    }

    public var body: some View {
        switch family {
        case .accessoryCircular:
            lockCircularView
        case .accessoryRectangular:
            lockRectangularView
        case .systemSmall:
            homeSmallView
        default:
            homeMediumView
        }
    }

    private var lockCircularView: some View {
        ZStack {
            Circle()
                .strokeBorder(lineWidth: 3)
                .opacity(0.35)
            if entry.isPoeticPrivacy {
                VStack(spacing: 1) {
                    Image(systemName: "leaf.fill")
                        .font(.system(size: 14, weight: .semibold))
                    Text(entry.brandTitle)
                        .font(.system(size: 10, weight: .bold))
                }
            } else {
                VStack(spacing: 0) {
                    Text("\(entry.snapshot.score)")
                        .font(.system(size: 18, weight: .bold, design: .rounded))
                        .monospacedDigit()
                    Text(entry.brandTitle)
                        .font(.system(size: 9, weight: .medium))
                        .opacity(0.8)
                }
            }
        }
        .accessibilityLabel("\(entry.brandTitle) \(entry.snapshot.stateBadge)")
    }

    private var lockRectangularView: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(spacing: 4) {
                Text("\(entry.brandTitle) · \(entry.snapshot.solarTerm)")
                    .font(.system(size: 11, weight: .semibold))
                    .lineLimit(1)
                Spacer(minLength: 2)
                if !entry.isPoeticPrivacy {
                    Text("\(entry.snapshot.score)")
                        .font(.system(size: 11, weight: .bold, design: .rounded))
                        .monospacedDigit()
                }
            }
            Text(entry.snapshot.poeticLine)
                .font(.system(size: 12, weight: .medium))
                .lineLimit(2)
            Text(entry.isPoeticPrivacy ? entry.snapshot.stateBadge : entry.snapshot.metricsLine)
                .font(.system(size: 10))
                .opacity(0.75)
                .lineLimit(1)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var homeSmallView: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(entry.brandTitle)
                    .font(.caption.bold())
                Spacer()
                Text(entry.snapshot.solarTerm)
                    .font(.caption2.weight(.semibold))
                    .padding(.horizontal, 6)
                    .padding(.vertical, 2)
                    .background(Capsule().opacity(0.14))
            }

            Spacer(minLength: 4)

            if entry.isPoeticPrivacy {
                Text(entry.snapshot.poeticLine)
                    .font(.subheadline.weight(.semibold))
                    .lineLimit(3)
            } else {
                Text("\(entry.snapshot.score)")
                    .font(.system(size: 32, weight: .bold, design: .rounded))
                    .monospacedDigit()
                Text(entry.snapshot.stateBadge)
                    .font(.caption.weight(.medium))
                    .opacity(0.8)
            }

            Text(entry.isPoeticPrivacy ? entry.snapshot.stateBadge : entry.snapshot.metricsLine)
                .font(.caption2)
                .opacity(0.75)
                .lineLimit(1)
        }
        .padding(14)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
    }

    private var homeMediumView: some View {
        HStack(spacing: 14) {
            VStack(alignment: .leading, spacing: 6) {
                Text("\(entry.brandTitle) · \(entry.snapshot.solarTerm)")
                    .font(.caption.bold())
                if entry.isPoeticPrivacy {
                    Text(entry.snapshot.stateBadge)
                        .font(.title3.bold())
                } else {
                    Text("\(entry.snapshot.score) / 100")
                        .font(.title2.bold())
                        .monospacedDigit()
                }
                Spacer(minLength: 2)
                Text(entry.snapshot.metricsLine)
                    .font(.caption2)
                    .opacity(0.78)
                    .lineLimit(2)
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            VStack(alignment: .leading, spacing: 6) {
                Text(entry.isChinese ? "今日顺时茶语" : "Daily Rhythm Cue")
                    .font(.caption2.weight(.semibold))
                    .opacity(0.7)
                Text(entry.snapshot.poeticLine)
                    .font(.subheadline.weight(.medium))
                    .lineLimit(4)
                Spacer(minLength: 2)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(16)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
    }
}

public struct QiSanctuaryWidget: Widget {
    public static let kind = "eu.org.cluas.qi.sanctuaryWidget"

    public init() {}

    public var body: some WidgetConfiguration {
        StaticConfiguration(kind: Self.kind, provider: QiTimelineProvider()) { entry in
            QiSanctuaryWidgetView(entry: entry)
        }
        .configurationDisplayName("Qi Sanctuary / 知气灵境")
        .description("Gentle daily rhythm and poetic sleep/HRV cue synced privately via local App Group.")
        .supportedFamilies([
            .accessoryCircular,
            .accessoryRectangular,
            .systemSmall,
            .systemMedium,
        ])
    }
}
#endif
