import Foundation
import Vision
#if canImport(AppKit)
import AppKit
#endif
#if canImport(FoundationModels)
import FoundationModels
#endif

struct ProbeOutput: Codable {
    let platform: String
    let visionOcrAvailable: Bool
    let visionRevision: Int
    let foundationModelsSupported: Bool
    let foundationModelsStatus: String
}

struct OcrOutput: Codable {
    let ok: Bool
    let text: String
    let lineCount: Int
    let error: String?
}

func emitJson<T: Encodable>(_ value: T) {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.withoutEscapingSlashes]
    if let data = try? encoder.encode(value),
       let str = String(data: data, encoding: .utf8) {
        print(str)
    }
}

func handleProbe() {
    var fmSupported = false
    var fmStatus = "os_not_supported"

    #if canImport(FoundationModels)
    if #available(macOS 26.0, iOS 26.0, *) {
        let avail = SystemLanguageModel.default.availability
        switch avail {
        case .available:
            fmSupported = true
            fmStatus = "available"
        case .unavailable(let reason):
            fmSupported = false
            fmStatus = "unavailable:\(reason)"
        @unknown default:
            fmSupported = false
            fmStatus = "unavailable:unknown"
        }
    }
    #endif

    emitJson(
        ProbeOutput(
            platform: "macos-apple-vision",
            visionOcrAvailable: true,
            visionRevision: VNRecognizeTextRequest.currentRevision,
            foundationModelsSupported: fmSupported,
            foundationModelsStatus: fmStatus
        )
    )
}

func handleOcr(imagePath: String) {
    let url = URL(fileURLWithPath: imagePath)
    guard FileManager.default.fileExists(atPath: url.path) else {
        emitJson(OcrOutput(ok: false, text: "", lineCount: 0, error: "图片文件不存在: \(imagePath)"))
        exit(1)
    }

    var cgImage: CGImage?
    #if canImport(AppKit)
    if let nsImage = NSImage(contentsOf: url) {
        var rect = NSRect(origin: .zero, size: nsImage.size)
        cgImage = nsImage.cgImage(forProposedRect: &rect, context: nil, hints: nil)
    }
    #endif

    let requestHandler: VNImageRequestHandler
    if let cg = cgImage {
        requestHandler = VNImageRequestHandler(cgImage: cg, options: [:])
    } else {
        requestHandler = VNImageRequestHandler(url: url, options: [:])
    }

    let request = VNRecognizeTextRequest()
    request.recognitionLevel = .accurate
    request.usesLanguageCorrection = true
    request.recognitionLanguages = ["zh-Hans", "en-US"]

    do {
        try requestHandler.perform([request])
        let observations = request.results ?? []
        // 按图像从上到下分组拼接同一行文本，保留表格结构给 Rust 解析器
        let sorted = observations.sorted { a, b in
            if abs(a.boundingBox.midY - b.boundingBox.midY) > 0.015 {
                return a.boundingBox.midY > b.boundingBox.midY
            }
            return a.boundingBox.minX < b.boundingBox.minX
        }
        var rows: [[(CGFloat, String)]] = []
        for obs in sorted {
            guard let top = obs.topCandidates(1).first else { continue }
            let y = obs.boundingBox.midY
            if let lastIdx = rows.indices.last,
               let firstInRow = rows[lastIdx].first,
               abs(firstInRow.0 - y) <= 0.018 {
                rows[lastIdx].append((y, top.string))
            } else {
                rows.append([(y, top.string)])
            }
        }
        let lines = rows.map { row in
            row.map { $0.1 }.joined(separator: " ")
        }
        emitJson(OcrOutput(ok: true, text: lines.joined(separator: "\n"), lineCount: lines.count, error: nil))
    } catch {
        emitJson(OcrOutput(ok: false, text: "", lineCount: 0, error: "Vision OCR 识别失败: \(error.localizedDescription)"))
        exit(2)
    }
}

let args = CommandLine.arguments
if args.count >= 2 && args[1] == "probe" {
    handleProbe()
} else if args.count >= 3 && args[1] == "ocr" {
    handleOcr(imagePath: args[2])
} else {
    fputs("Usage: qi-vision-helper <probe | ocr IMAGE_PATH>\n", stderr)
    exit(64)
}
