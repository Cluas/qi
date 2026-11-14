import Foundation
import StoreKit
#if canImport(UIKit)
import UIKit
#endif
#if canImport(WebKit)
import WebKit
#endif

/// 知气 / Qi Apple StoreKit 2 商品标识定义（与 QiAtelier.storekit 及云端控制面对齐）
public enum QiProductID {
    public static let proMonthly = "com.qi.atelier.pro_monthly"
    public static let seasonal = "com.qi.atelier.seasonal"
    public static let proAnnual = "com.qi.atelier.pro_annual"
    public static let duo = "com.qi.atelier.duo"
    public static let family = "com.qi.atelier.family"
    public static let lifetimeLocal = "com.qi.atelier.lifetime_local"
    /// 商业消耗型礼赠商品：窗台添柴礼赠券 · 赠予社区漂流席位 / Hearth Gift Pass · Gift 1 Community Seat
    public static let hearthGiftPass = "com.qi.hearth_gift_pass"

    public static let subscriptionIDs: [String] = [
        proMonthly,
        seasonal,
        proAnnual,
        duo,
        family,
    ]

    public static let allProductIDs: [String] = [
        proMonthly,
        seasonal,
        proAnnual,
        duo,
        family,
        lifetimeLocal,
        hearthGiftPass,
    ]
}

public enum StoreKitBridgeError: Error, LocalizedError {
    case productNotFound(String)
    case verificationFailed(String)
    case missingCloudConfiguration
    case invalidServerResponse(Int, String)

    public var errorDescription: String? {
        switch self {
        case .productNotFound(let productId):
            return "App Store product not found: \(productId)"
        case .verificationFailed(let detail):
            return "StoreKit transaction verification failed: \(detail)"
        case .missingCloudConfiguration:
            return "Missing cloud base URL or bearer token for Apple receipt verification"
        case .invalidServerResponse(let status, let body):
            return "Cloud verify failed with HTTP \(status): \(body)"
        }
    }
}

public struct StoreProductDTO: Codable, Sendable {
    public let product_id: String
    public let display_name: String
    public let description: String
    public let display_price: String
    public let price_decimal: String
    public let product_type: String
    public let is_consumable: Bool
    public let subscription_period_unit: String?
    public let subscription_period_value: Int?
}

/// 可直接提交至 `POST /v1/billing/apple/verify` 的 StoreKit 2 JWS 交易载荷
public struct VerifiedTransactionPayload: Codable, Sendable {
    public let jws_representation: String
    public let product_id: String
    public let transaction_id: String
    public let original_transaction_id: String
    public let environment: String
    public let bundle_id: String
    public let purchase_date_epoch: Int64
    public let expires_date_epoch: Int64?
    public let revocation_date_epoch: Int64?
    public let quantity: Int
    public let is_hearth_gift_pass: Bool
}

public enum StorePurchaseOutcome: Sendable {
    case verified(VerifiedTransactionPayload)
    case userCancelled
    case pending
}

@available(iOS 15.0, macOS 12.0, *)
public final class StoreKitBridge {
    public static let shared = StoreKitBridge()
    public static let expectedBundleID = "eu.org.cluas.qi"

    private var updatesTask: Task<Void, Never>?
    private let lock = NSLock()

    public init() {}

    deinit {
        stopTransactionUpdatesListener()
    }

    // MARK: - Product Loading

    public func fetchProducts(
        productIds: [String] = QiProductID.allProductIDs
    ) async throws -> [Product] {
        let products = try await Product.products(for: productIds)
        return products.sorted { lhs, rhs in
            let li = QiProductID.allProductIDs.firstIndex(of: lhs.id) ?? Int.max
            let ri = QiProductID.allProductIDs.firstIndex(of: rhs.id) ?? Int.max
            return li < ri
        }
    }

    public func fetchProductDTOs(
        productIds: [String] = QiProductID.allProductIDs
    ) async throws -> [StoreProductDTO] {
        let products = try await fetchProducts(productIds: productIds)
        return products.map(Self.makeDTO(from:))
    }

    public func fetchProductDTOsJSON(
        productIds: [String] = QiProductID.allProductIDs
    ) async throws -> String {
        let dtos = try await fetchProductDTOs(productIds: productIds)
        let data = try JSONEncoder().encode(dtos)
        return String(data: data, encoding: .utf8) ?? "[]"
    }

    // MARK: - Purchasing

    public func purchase(
        productId: String,
        appAccountToken: UUID? = nil
    ) async throws -> StorePurchaseOutcome {
        let products = try await Product.products(for: [productId])
        guard let product = products.first(where: { $0.id == productId }) else {
            throw StoreKitBridgeError.productNotFound(productId)
        }

        var options: Set<Product.PurchaseOption> = []
        if let token = appAccountToken {
            options.insert(.appAccountToken(token))
        }

        let result = try await product.purchase(options: options)
        switch result {
        case .success(let verificationResult):
            let payload = try Self.extractVerifiedPayload(from: verificationResult)
            if case .verified(let transaction) = verificationResult {
                await transaction.finish()
            }
            return .verified(payload)
        case .userCancelled:
            return .userCancelled
        case .pending:
            return .pending
        @unknown default:
            return .pending
        }
    }

    // MARK: - Entitlements & Restore Purchases

    public func currentEntitlements() async -> [VerifiedTransactionPayload] {
        var collected: [VerifiedTransactionPayload] = []
        for await result in Transaction.currentEntitlements {
            if let payload = try? Self.extractVerifiedPayload(from: result),
               payload.revocation_date_epoch == nil
            {
                collected.append(payload)
            }
        }
        return collected
    }

    /// 响应用户点击「恢复购买 / Restore purchases」：调用 AppStore.sync() 并导出当前有效交易 JWS 列表
    public func restorePurchases() async throws -> [VerifiedTransactionPayload] {
        try await AppStore.sync()
        return await currentEntitlements()
    }

    public func exportCurrentEntitlementsJSON() async throws -> String {
        let payloads = await currentEntitlements()
        let data = try JSONEncoder().encode(payloads)
        return String(data: data, encoding: .utf8) ?? "[]"
    }

    public func exportJWSJSON(for payload: VerifiedTransactionPayload) throws -> String {
        let data = try JSONEncoder().encode(payload)
        return String(data: data, encoding: .utf8) ?? "{}"
    }

    // MARK: - Transaction.updates Background Listener

    public func startTransactionUpdatesListener(
        onVerifiedTransaction: @escaping @Sendable (VerifiedTransactionPayload) async -> Void,
        onVerificationFailure: (@Sendable (Error) -> Void)? = nil
    ) {
        lock.lock()
        updatesTask?.cancel()
        let task = Task.detached(priority: .utility) {
            for await result in Transaction.updates {
                if Task.isCancelled { break }
                do {
                    let payload = try Self.extractVerifiedPayload(from: result)
                    await onVerifiedTransaction(payload)
                    if case .verified(let transaction) = result {
                        await transaction.finish()
                    }
                } catch {
                    onVerificationFailure?(error)
                }
            }
        }
        updatesTask = task
        lock.unlock()
    }

    public func stopTransactionUpdatesListener() {
        lock.lock()
        updatesTask?.cancel()
        updatesTask = nil
        lock.unlock()
    }

    // MARK: - Manage Subscriptions Helper

    #if canImport(UIKit) && !os(macOS)
    @MainActor
    public func showManageSubscriptions(in scene: UIWindowScene) async throws {
        try await AppStore.showManageSubscriptions(in: scene)
    }
    #endif

    // MARK: - Backend Verification Helper

    public func verifyWithCloudControlPlane(
        payload: VerifiedTransactionPayload,
        cloudBaseURL: String,
        bearerToken: String,
        companionLoyaltyDays: Int? = nil
    ) async throws -> Data {
        let trimmedURL = cloudBaseURL.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedToken = bearerToken.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedURL.isEmpty,
              !trimmedToken.isEmpty,
              let endpoint = URL(string: trimmedURL.trimmingCharacters(in: CharacterSet(charactersIn: "/")) + "/v1/billing/apple/verify")
        else {
            throw StoreKitBridgeError.missingCloudConfiguration
        }

        struct VerifyRequestBody: Codable {
            let jws_representation: String
            let product_id: String
            let transaction_id: String
            let original_transaction_id: String
            let environment: String
            let bundle_id: String
            let expires_date_epoch: Int64?
            let quantity: Int
            let companion_loyalty_days: Int?
        }

        let body = VerifyRequestBody(
            jws_representation: payload.jws_representation,
            product_id: payload.product_id,
            transaction_id: payload.transaction_id,
            original_transaction_id: payload.original_transaction_id,
            environment: payload.environment,
            bundle_id: payload.bundle_id,
            expires_date_epoch: payload.expires_date_epoch,
            quantity: payload.quantity,
            companion_loyalty_days: companionLoyaltyDays
        )

        var request = URLRequest(url: endpoint)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.setValue("Bearer \(trimmedToken)", forHTTPHeaderField: "Authorization")
        request.httpBody = try JSONEncoder().encode(body)

        let (data, response) = try await URLSession.shared.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw StoreKitBridgeError.invalidServerResponse(0, "Non-HTTP response")
        }
        guard (200...299).contains(http.statusCode) else {
            let text = String(data: data, encoding: .utf8) ?? ""
            throw StoreKitBridgeError.invalidServerResponse(http.statusCode, text)
        }
        return data
    }

    // MARK: - Internal Extraction Helpers

    private static func extractVerifiedPayload(
        from result: VerificationResult<Transaction>
    ) throws -> VerifiedTransactionPayload {
        switch result {
        case .unverified(_, let error):
            throw StoreKitBridgeError.verificationFailed(error.localizedDescription)
        case .verified(let transaction):
            let envLabel: String
            if #available(iOS 16.0, macOS 13.0, *) {
                envLabel = transaction.environment.rawValue
            } else {
                envLabel = transaction.environmentStringRepresentation
            }
            let expiresEpoch = transaction.expirationDate.map { Int64($0.timeIntervalSince1970) }
            let revocationEpoch = transaction.revocationDate.map { Int64($0.timeIntervalSince1970) }
            let purchaseEpoch = Int64(transaction.purchaseDate.timeIntervalSince1970)
            let bundleId = transaction.appBundleID.isEmpty ? expectedBundleID : transaction.appBundleID

            return VerifiedTransactionPayload(
                jws_representation: result.jwsRepresentation,
                product_id: transaction.productID,
                transaction_id: String(transaction.id),
                original_transaction_id: String(transaction.originalID),
                environment: envLabel,
                bundle_id: bundleId,
                purchase_date_epoch: purchaseEpoch,
                expires_date_epoch: expiresEpoch,
                revocation_date_epoch: revocationEpoch,
                quantity: transaction.purchasedQuantity,
                is_hearth_gift_pass: transaction.productID == QiProductID.hearthGiftPass
            )
        }
    }

    private static func makeDTO(from product: Product) -> StoreProductDTO {
        let typeLabel: String
        let isConsumable: Bool
        switch product.type {
        case .autoRenewable:
            typeLabel = "autoRenewable"
            isConsumable = false
        case .nonConsumable:
            typeLabel = "nonConsumable"
            isConsumable = false
        case .consumable:
            typeLabel = "consumable"
            isConsumable = true
        case .nonRenewable:
            typeLabel = "nonRenewable"
            isConsumable = false
        default:
            typeLabel = "unknown"
            isConsumable = false
        }

        let periodUnit: String?
        let periodValue: Int?
        if let period = product.subscription?.subscriptionPeriod {
            periodValue = period.value
            switch period.unit {
            case .day: periodUnit = "day"
            case .week: periodUnit = "week"
            case .month: periodUnit = "month"
            case .year: periodUnit = "year"
            @unknown default: periodUnit = nil
            }
        } else {
            periodUnit = nil
            periodValue = nil
        }

        return StoreProductDTO(
            product_id: product.id,
            display_name: product.displayName,
            description: product.description,
            display_price: product.displayPrice,
            price_decimal: NSDecimalNumber(decimal: product.price).stringValue,
            product_type: typeLabel,
            is_consumable: isConsumable,
            subscription_period_unit: periodUnit,
            subscription_period_value: periodValue
        )
    }
}

#if canImport(WebKit) && (os(iOS) || os(macOS))
/// 将原生 StoreKit 2 购买与恢复购买能力注入 WKWebView (`globalThis.__qiStoreKitBridge`)。
@available(iOS 15.0, macOS 12.0, *)
public final class StoreKitWebBridge: NSObject, WKScriptMessageHandlerWithReply {
    public static let messageHandlerName = "qiStoreKitBridge"
    public static let shared = StoreKitWebBridge()

    private static let injectionScriptSource = """
    (function() {
      if (globalThis.__qiStoreKitBridge) { return; }
      function postBridge(action, payload) {
        return window.webkit.messageHandlers.qiStoreKitBridge.postMessage({
          action: action,
          payload: payload || {}
        });
      }
      globalThis.__qiStoreKitBridge = {
        purchaseProduct: function(productId) {
          return postBridge("purchaseProduct", { productId: productId });
        },
        restorePurchases: function() {
          return postBridge("restorePurchases", {});
        },
        fetchProducts: function(productIds) {
          return postBridge("fetchProducts", { productIds: productIds || [] });
        }
      };
    })();
    """

    public override init() {
        super.init()
    }

    @MainActor
    public func install(into userContentController: WKUserContentController) {
        userContentController.removeScriptMessageHandler(forName: Self.messageHandlerName)
        userContentController.addScriptMessageHandler(
            self,
            contentWorld: .page,
            name: Self.messageHandlerName
        )
        let script = WKUserScript(
            source: Self.injectionScriptSource,
            injectionTime: .atDocumentStart,
            forMainFrameOnly: true
        )
        userContentController.addUserScript(script)
    }

    public func userContentController(
        _ userContentController: WKUserContentController,
        didReceive message: WKScriptMessage,
        replyHandler: @escaping (Any?, String?) -> Void
    ) {
        guard let body = message.body as? [String: Any],
              let action = body["action"] as? String else {
            replyHandler(nil, "Invalid StoreKit bridge message payload")
            return
        }
        let payload = body["payload"] as? [String: Any] ?? [:]

        Task {
            do {
                let resultJSON: String
                switch action {
                case "purchaseProduct":
                    guard let productId = (payload["productId"] as? String) ?? (body["productId"] as? String),
                          !productId.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
                        await MainActor.run { replyHandler(nil, "Missing productId") }
                        return
                    }
                    let outcome = try await StoreKitBridge.shared.purchase(productId: productId)
                    switch outcome {
                    case .verified(let tx):
                        resultJSON = try StoreKitBridge.shared.exportJWSJSON(for: tx)
                    case .userCancelled:
                        resultJSON = "{\"cancelled\":true}"
                    case .pending:
                        resultJSON = "{\"pending\":true}"
                    }
                case "restorePurchases":
                    let txs = try await StoreKitBridge.shared.restorePurchases()
                    let data = try JSONEncoder().encode(txs)
                    resultJSON = String(data: data, encoding: .utf8) ?? "[]"
                case "fetchProducts":
                    let requested = payload["productIds"] as? [String] ?? []
                    let ids = requested.isEmpty ? QiProductID.allProductIDs : requested
                    resultJSON = try await StoreKitBridge.shared.fetchProductDTOsJSON(productIds: ids)
                default:
                    await MainActor.run {
                        replyHandler(nil, "Unsupported StoreKit bridge action: \(action)")
                    }
                    return
                }
                await MainActor.run {
                    replyHandler(resultJSON, nil)
                }
            } catch {
                await MainActor.run {
                    replyHandler(nil, error.localizedDescription)
                }
            }
        }
    }
}
#endif
