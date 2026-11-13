import AppKit
import CoreGraphics
import Foundation

func saveBitmap(_ rep: NSBitmapImageRep, to path: String) {
    let url = URL(fileURLWithPath: path)
    try? FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
    guard let data = rep.representation(using: .png, properties: [:]) else {
        fatalError("PNG encoding failed for \(path)")
    }
    try! data.write(to: url)
    print("Generated \(path) (\(data.count) bytes)")
}

func renderIcon(to path: String) {
    let size = 1024
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: size,
        pixelsHigh: size,
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    )!
    NSGraphicsContext.saveGraphicsState()
    let ctx = NSGraphicsContext(bitmapImageRep: rep)!
    NSGraphicsContext.current = ctx
    let cg = ctx.cgContext

    // 宣纸暖底
    cg.setFillColor(NSColor(calibratedRed: 0.969, green: 0.961, blue: 0.941, alpha: 1.0).cgColor)
    cg.fill(CGRect(x: 0, y: 0, width: size, height: size))

    // 内润柔和同心浅碧环
    let center = CGPoint(x: 512, y: 512)
    cg.setStrokeColor(NSColor(calibratedRed: 0.894, green: 0.875, blue: 0.827, alpha: 1.0).cgColor)
    cg.setLineWidth(64)
    cg.addArc(center: center, radius: 330, startAngle: 0, endAngle: .pi * 2, clockwise: false)
    cg.strokePath()

    // 宋韵青玉主力元气弧 (约 82% 电量感)
    cg.setStrokeColor(NSColor(calibratedRed: 0.165, green: 0.435, blue: 0.357, alpha: 1.0).cgColor)
    cg.setLineWidth(64)
    cg.setLineCap(.round)
    cg.addArc(center: center, radius: 330, startAngle: .pi * 0.5, endAngle: -.pi * 1.15, clockwise: true)
    cg.strokePath()

    // 琥珀晨光节律游标点
    let dotAngle: CGFloat = -.pi * 1.15
    let dotPos = CGPoint(x: 512 + cos(dotAngle) * 330, y: 512 + sin(dotAngle) * 330)
    cg.setFillColor(NSColor(calibratedRed: 0.722, green: 0.490, blue: 0.169, alpha: 1.0).cgColor)
    cg.fillEllipse(in: CGRect(x: dotPos.x - 42, y: dotPos.y - 42, width: 84, height: 84))

    // 中心「气」字印章
    let title = "气" as NSString
    let font = NSFont(name: "PingFangSC-Semibold", size: 320) ?? NSFont.systemFont(ofSize: 320, weight: .semibold)
    let attrs: [NSAttributedString.Key: Any] = [
        .font: font,
        .foregroundColor: NSColor(calibratedRed: 0.094, green: 0.102, blue: 0.094, alpha: 1.0),
    ]
    let textSize = title.size(withAttributes: attrs)
    title.draw(at: NSPoint(x: (1024 - textSize.width) / 2, y: (1024 - textSize.height) / 2 - 12), withAttributes: attrs)

    NSGraphicsContext.restoreGraphicsState()
    saveBitmap(rep, to: path)
}

func renderSampleLab(to path: String) {
    let w = 1280
    let h = 920
    let rep = NSBitmapImageRep(
        bitmapDataPlanes: nil,
        pixelsWide: w,
        pixelsHigh: h,
        bitsPerSample: 8,
        samplesPerPixel: 4,
        hasAlpha: true,
        isPlanar: false,
        colorSpaceName: .deviceRGB,
        bytesPerRow: 0,
        bitsPerPixel: 0
    )!
    NSGraphicsContext.saveGraphicsState()
    let ctx = NSGraphicsContext(bitmapImageRep: rep)!
    NSGraphicsContext.current = ctx
    let cg = ctx.cgContext

    cg.setFillColor(NSColor.white.cgColor)
    cg.fill(CGRect(x: 0, y: 0, width: w, height: h))

    let lines = [
        "某某市第一人民医院体检中心 检验报告单",
        "姓名：陈默 身份证号：31010419920518432X 手机：13912345678",
        "尿酸(UA) 468 umol/L 208-428",
        "低密度脂蛋白胆固醇(LDL-C) 3.62 mmol/L 0.00-3.40",
        "谷丙转氨酶(ALT) 56 U/L 9-50",
        "25-羟维生素D(25-OH-VD) 18.5 ng/mL 30.0-100.0",
        "空腹血糖(GLU) 5.08 mmol/L 3.90-6.10",
    ]
    let font = NSFont(name: "PingFangSC-Regular", size: 32) ?? NSFont.systemFont(ofSize: 32)
    let attrs: [NSAttributedString.Key: Any] = [
        .font: font,
        .foregroundColor: NSColor.black,
    ]
    var y: CGFloat = CGFloat(h) - 110
    for line in lines {
        (line as NSString).draw(at: NSPoint(x: 64, y: y), withAttributes: attrs)
        y -= 104
    }

    NSGraphicsContext.restoreGraphicsState()
    saveBitmap(rep, to: path)
}

let root = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "."
renderIcon(to: "\(root)/assets/icon-1024.png")
renderSampleLab(to: "\(root)/tests/fixtures/sample_lab_report.png")
