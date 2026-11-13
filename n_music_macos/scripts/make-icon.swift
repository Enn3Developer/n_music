// Draws the app icon's sizes from the Android launcher artwork (the Play Store PNG, a full-bleed
// pixel-art logo on black), on the macOS icon grid: a rounded square of 824 points in 1024 with
// the drop shadow macOS icons carry, the artwork inside, scaled without smoothing so its pixels
// stay sharp. It also writes the sidebar logo (the artwork alone; the view rounds it).
//
//   swift n_music_macos/scripts/make-icon.swift \
//       n_music_android/android/src/main/ic_launcher-playstore.png \
//       n_music_macos/NMusic/Assets.xcassets
import AppKit
import CoreGraphics

let arguments = CommandLine.arguments
guard arguments.count == 3,
      let source = NSImage(contentsOfFile: arguments[1])?
          .cgImage(forProposedRect: nil, context: nil, hints: nil)
else {
    FileHandle.standardError.write("usage: make-icon.swift <artwork.png> <Assets.xcassets>\n".data(using: .utf8)!)
    exit(1)
}
let catalog = URL(fileURLWithPath: arguments[2])
let iconSet = catalog.appendingPathComponent("AppIcon.appiconset")
let logoSet = catalog.appendingPathComponent("Logo.imageset")
for folder in [iconSet, logoSet] {
    try! FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
}

func context(_ pixels: Int) -> CGContext {
    CGContext(
        data: nil, width: pixels, height: pixels, bitsPerComponent: 8, bytesPerRow: 0,
        space: CGColorSpace(name: CGColorSpace.sRGB)!,
        bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
    )!
}

func png(_ context: CGContext) -> Data {
    NSBitmapImageRep(cgImage: context.makeImage()!).representation(using: .png, properties: [:])!
}

func icon(pixels: Int) -> Data {
    let ctx = context(pixels)
    ctx.scaleBy(x: CGFloat(pixels) / 1024, y: CGFloat(pixels) / 1024)
    let body = CGRect(x: 100, y: 100, width: 824, height: 824)
    let shape = CGPath(roundedRect: body, cornerWidth: 185, cornerHeight: 185, transform: nil)
    ctx.saveGState()
    ctx.setShadow(offset: CGSize(width: 0, height: -10), blur: 28, color: CGColor(gray: 0, alpha: 0.35))
    ctx.addPath(shape)
    ctx.setFillColor(CGColor(gray: 0, alpha: 1))
    ctx.fillPath()
    ctx.restoreGState()
    ctx.addPath(shape)
    ctx.clip()
    ctx.interpolationQuality = .none
    // The artwork's own black ground meets the body's; inset so the arc clears the corners.
    ctx.draw(source, in: body.insetBy(dx: 56, dy: 56))
    return png(ctx)
}

func logo(pixels: Int) -> Data {
    let ctx = context(pixels)
    ctx.interpolationQuality = .none
    ctx.draw(source, in: CGRect(x: 0, y: 0, width: pixels, height: pixels))
    return png(ctx)
}

var images: [[String: String]] = []
for points in [16, 32, 128, 256, 512] {
    for scale in [1, 2] {
        let name = "icon_\(points)x\(points)\(scale == 2 ? "@2x" : "").png"
        try! icon(pixels: points * scale).write(to: iconSet.appendingPathComponent(name))
        images.append(["idiom": "mac", "scale": "\(scale)x", "size": "\(points)x\(points)", "filename": name])
    }
}
func contents(_ images: [[String: String]]) -> Data {
    try! JSONSerialization.data(
        withJSONObject: ["images": images, "info": ["author": "xcode", "version": 1]],
        options: [.prettyPrinted, .sortedKeys]
    )
}
try! contents(images).write(to: iconSet.appendingPathComponent("Contents.json"))

// The sidebar shows it at 28 points (36 on the import page): 64 and 128 pixels cover both.
var logos: [[String: String]] = []
for (scale, pixels) in [(1, 64), (2, 128)] {
    let name = "logo@\(scale)x.png"
    try! logo(pixels: pixels).write(to: logoSet.appendingPathComponent(name))
    logos.append(["idiom": "universal", "scale": "\(scale)x", "filename": name])
}
try! contents(logos).write(to: logoSet.appendingPathComponent("Contents.json"))
print("Wrote \(iconSet.path) and \(logoSet.path)")
