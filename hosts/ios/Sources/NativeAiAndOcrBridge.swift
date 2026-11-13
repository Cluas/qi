import Foundation
import Vision
#if canImport(UIKit)
import UIKit
#endif
#if canImport(FoundationModels)
import FoundationModels
#endif

/// iOS 原生离线 OCR（Vision）与系统内置 Apple Intelligence (`FoundationModels`) 桥接
public final class NativeAiAndOcrBridge {
    public init() {}

    public func isFoundationModelsAvailable() -> Bool {
        #if canImport(FoundationModels)
        if #available(iOS 26.0, macOS 26.0, *) {
            if case .available = SystemLanguageModel.default.availability {
                return true
            }
        }
        #endif
        return false
    }

    /// 0 流量系统离线文字识别：识别体检单后交由 Rust 做页眉 PII 遮盖与区间重算
    public func recognizeTextOffline(imageData: Data, completion: @escaping (Result<String, Error>) -> Void) {
        let request = VNRecognizeTextRequest { req, err in
            if let err = err {
                completion(.failure(err))
                return
            }
            let lines = (req.results as? [VNRecognizedTextObservation] ?? [])
                .compactMap { $0.topCandidates(1).first?.string }
            completion(.success(lines.joined(separator: "\n")))
        }
        request.recognitionLevel = .accurate
        request.usesLanguageCorrection = true
        request.recognitionLanguages = ["zh-Hans", "en-US"]

        DispatchQueue.global(qos: .userInitiated).async {
            let handler = VNImageRequestHandler(data: imageData, options: [:])
            do {
                try handler.perform([request])
            } catch {
                completion(.failure(error))
            }
        }
    }
}
