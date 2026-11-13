import AppKit
import ImageIO

/// A decoded picture, handed between threads (CGImage is immutable).
struct Picture: @unchecked Sendable {
    let image: CGImage
}

/// Covers, decoded off the main thread at the size they are shown at and kept in memory.
///
/// ImageIO decodes a thumbnail straight to the size asked for
/// (`CGImageSourceCreateThumbnailAtIndex`), so a cover shown at 40 points costs a 40-point
/// bitmap, never the full picture. The cache is bounded by decoded bytes, and the system takes
/// it back under memory pressure. Use `CoverView` in views; this is for anything else.
final class Artwork: @unchecked Sendable {
    static let shared = Artwork()

    private let cache: NSCache<NSString, CGImage> = {
        let cache = NSCache<NSString, CGImage>()
        cache.totalCostLimit = 96 << 20
        return cache
    }()

    /// A cached thumbnail, without decoding: lets a view show it on its first frame.
    func cached(path: String, pixels: Int) -> Picture? {
        cache.object(forKey: Self.key(path, pixels)).map(Picture.init)
    }

    /// The image file at `path` (a cover thumbnail from the library), decoded for `pixels`
    /// on its longer side.
    func thumbnail(path: String, pixels: Int) async -> Picture? {
        let key = Self.key(path, pixels)
        if let image = cache.object(forKey: key) {
            return Picture(image: image)
        }
        let picture = await Task.detached(priority: .utility) {
            Self.decode(CGImageSourceCreateWithURL(URL(fileURLWithPath: path) as CFURL, nil), pixels: pixels)
        }.value
        if let picture {
            cache.setObject(picture.image, forKey: key, cost: picture.image.bytesPerRow * picture.image.height)
        }
        return picture
    }

    /// The picture embedded in the file of `locator` at full quality, decoded for `pixels`, for
    /// the player's large covers. It reads the file, so it is not cached: the caller keeps it
    /// while it shows the track.
    func full(_ locator: Locator, pixels: Int, core: Core) async -> Picture? {
        await Task.detached(priority: .utility) {
            guard let data = core.coverArt(locator: locator) else { return nil }
            return Self.decode(CGImageSourceCreateWithData(data as CFData, nil), pixels: pixels)
        }.value
    }

    /// Decodes an image file's data (e.g. the app's own resources) at `pixels`.
    static func decode(data: Data, pixels: Int) -> Picture? {
        decode(CGImageSourceCreateWithData(data as CFData, nil), pixels: pixels)
    }

    private static func key(_ path: String, _ pixels: Int) -> NSString {
        "\(pixels)\u{1}\(path)" as NSString
    }

    private static func decode(_ source: CGImageSource?, pixels: Int) -> Picture? {
        guard let source else { return nil }
        let options: [CFString: Any] = [
            kCGImageSourceCreateThumbnailFromImageAlways: true,
            kCGImageSourceCreateThumbnailWithTransform: true,
            kCGImageSourceShouldCacheImmediately: true,
            kCGImageSourceThumbnailMaxPixelSize: max(pixels, 1),
        ]
        return CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary).map(Picture.init)
    }
}
