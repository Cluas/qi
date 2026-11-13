import Foundation
import HealthKit

/// iOS HealthKit 零过桥同步适配器（源自 Apache-2.0 PulsHealthSync 理念重构）：
/// 1. 仅向用户请求 6 项消费级核心指标，绝不暴露 Raw / Aggregate 与 80 项指标开关；
/// 2. 严格区分 Apple Watch 的 SDNN (`hrv_sdnn_ms`)，绝不与 Garmin/Oura RMSSD 混算；
/// 3. 原始样本直写 App Group JSONL 文件供 Rust Core 合并，JS 视图层不接触样本。
public final class HealthKitSyncBridge {
    public static let appGroupId = "group.eu.org.cluas.qi"
    private let store = HKHealthStore()

    public init() {}

    public func requestMinimalPermissions(completion: @escaping (Bool, Error?) -> Void) {
        guard HKHealthStore.isHealthDataAvailable() else {
            completion(false, nil)
            return
        }
        let readTypes: Set<HKObjectType> = [
            HKObjectType.categoryType(forIdentifier: .sleepAnalysis)!,
            HKObjectType.quantityType(forIdentifier: .stepCount)!,
            HKObjectType.quantityType(forIdentifier: .heartRate)!,
            HKObjectType.quantityType(forIdentifier: .restingHeartRate)!,
            HKObjectType.quantityType(forIdentifier: .heartRateVariabilitySDNN)!,
            HKObjectType.quantityType(forIdentifier: .activeEnergyBurned)!,
            HKObjectType.workoutType(),
        ]
        store.requestAuthorization(toShare: nil, read: readTypes, completion: completion)
    }

    public func exportRecentDaysToAppGroup(days: Int = 35, completion: @escaping (URL?, Error?) -> Void) {
        guard let container = FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: Self.appGroupId) else {
            completion(nil, NSError(domain: "QiHealthKit", code: 1, userInfo: [NSLocalizedDescriptionKey: "未配置 App Group 容器"]))
            return
        }
        let targetFile = container.appendingPathComponent("hk_delta_feed.jsonl")
        let end = Date()
        let start = Calendar.current.date(byAdding: .day, value: -days, to: end) ?? end
        let predicate = HKQuery.predicateForSamples(withStart: start, end: end, options: .strictStartDate)

        var lines: [String] = []
        let lock = NSLock()
        let group = DispatchGroup()
        let df = ISO8601DateFormatter()
        df.formatOptions = [.withFullDate]

        func appendDailyAvg(id: HKQuantityTypeIdentifier, canonical: String, unit: HKUnit, unitLabel: String) {
            guard let qtyType = HKQuantityType.quantityType(forIdentifier: id) else { return }
            group.enter()
            let query = HKStatisticsCollectionQuery(
                quantityType: qtyType,
                quantitySamplePredicate: predicate,
                options: id == .stepCount ? .cumulativeSum : .discreteAverage,
                anchorDate: Calendar.current.startOfDay(for: end),
                intervalComponents: DateComponents(day: 1)
            )
            query.initialResultsHandler = { _, collection, _ in
                defer { group.leave() }
                collection?.enumerateStatistics(from: start, to: end) { stats, _ in
                    let q = id == .stepCount ? stats.sumQuantity() : stats.averageQuantity()
                    if let val = q?.doubleValue(for: unit) {
                        let dateStr = String(df.string(from: stats.startDate).prefix(10))
                        let line = "{\"kind\":\"metric\",\"date\":\"\(dateStr)\",\"metric\":\"\(canonical)\",\"source\":\"apple_health\",\"value\":\(round(val * 100) / 100),\"unit\":\"\(unitLabel)\"}"
                        lock.lock()
                        lines.append(line)
                        lock.unlock()
                    }
                }
            }
            store.execute(query)
        }

        appendDailyAvg(id: .heartRateVariabilitySDNN, canonical: "hrv_sdnn_ms", unit: HKUnit.secondUnit(with: .milli), unitLabel: "ms")
        appendDailyAvg(id: .restingHeartRate, canonical: "resting_heart_rate_bpm", unit: HKUnit.count().unitDivided(by: .minute()), unitLabel: "bpm")
        appendDailyAvg(id: .stepCount, canonical: "steps", unit: HKUnit.count(), unitLabel: "步")

        group.notify(queue: .global(qos: .utility)) {
            do {
                let payload = lines.joined(separator: "\n")
                try payload.write(to: targetFile, atomically: true, encoding: .utf8)
                completion(targetFile, nil)
            } catch {
                completion(nil, error)
            }
        }
    }
}
