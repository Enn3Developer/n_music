import SwiftUI

/// A track's or a group's cover thumbnail (a file the library made), decoded off the main thread
/// at the size it shows at (`Artwork`). While it loads: a `raised` tile; without a cover: the
/// tile with `placeholder` (an SF Symbol) at 45 % of the size, at most 72 points. A new size of
/// the same cover keeps showing the picture it has, scaled, until the new one is decoded.
struct CoverView: View {
    let path: String?
    var size: CGFloat
    var radius: CGFloat = 4
    var placeholder = "music.note"

    @Environment(\.displayScale) private var scale
    @State private var picture: Picture?
    /// The cover `picture` is of.
    @State private var shownPath: String?
    @State private var loaded = false

    var body: some View {
        ZStack {
            Rectangle().fill(Theme.raised)
            if let picture {
                Image(decorative: picture.image, scale: 1)
                    .resizable()
                    .interpolation(.high)
                    .aspectRatio(contentMode: .fill)
            } else if loaded || path == nil {
                Image(systemName: placeholder)
                    .font(.system(size: min(size * 0.45, 72) * 0.6, weight: .regular))
                    .foregroundStyle(Theme.text3)
            }
        }
        .frame(width: size, height: size)
        .clipShape(RoundedRectangle(cornerRadius: radius, style: .continuous))
        .task(id: TaskKey(path: path, pixels: pixels)) {
            guard let path else {
                picture = nil
                shownPath = nil
                loaded = true
                return
            }
            if let cached = Artwork.shared.cached(path: path, pixels: pixels) {
                picture = cached
                shownPath = path
                loaded = true
                return
            }
            if shownPath != path {
                picture = nil
                shownPath = nil
                loaded = false
            }
            let decoded = await Artwork.shared.thumbnail(path: path, pixels: pixels)
            guard !Task.isCancelled else { return }
            picture = decoded
            shownPath = path
            loaded = true
        }
    }

    private var pixels: Int { Int((size * scale).rounded(.up)) }

    private struct TaskKey: Equatable {
        let path: String?
        let pixels: Int
    }
}

/// Up to four covers in a 2 × 2 mosaic, as playlists and genres show them; fewer than four
/// show the first alone.
struct MosaicView: View {
    let covers: [String]
    var size: CGFloat
    var radius: CGFloat = 6
    var placeholder = "music.note.list"

    var body: some View {
        Group {
            if covers.count < 4 {
                CoverView(path: covers.first, size: size, radius: 0, placeholder: placeholder)
            } else {
                Grid(horizontalSpacing: 0, verticalSpacing: 0) {
                    GridRow {
                        CoverView(path: covers[0], size: size / 2, radius: 0)
                        CoverView(path: covers[1], size: size / 2, radius: 0)
                    }
                    GridRow {
                        CoverView(path: covers[2], size: size / 2, radius: 0)
                        CoverView(path: covers[3], size: size / 2, radius: 0)
                    }
                }
            }
        }
        .frame(width: size, height: size)
        .clipShape(RoundedRectangle(cornerRadius: radius, style: .continuous))
    }
}

/// The app's logo (the launcher artwork), its corners rounded like the tiles around it.
struct LogoView: View {
    var size: CGFloat = 28
    var radius: CGFloat = 8

    var body: some View {
        Image("Logo")
            .resizable()
            .interpolation(.none)
            .frame(width: size, height: size)
            .clipShape(RoundedRectangle(cornerRadius: radius, style: .continuous))
            .accessibilityHidden(true)
    }
}

/// The app's monochrome icon (Qt's MonoLogo, the Android launcher's themed layer): equaliser
/// bars around an N on a 50-unit grid, the bars in `border`, the N in `text3`. A cover tile
/// without a cover shows it at the tile's size.
struct MonoLogo: View {
    var size: CGFloat

    var body: some View {
        Canvas { context, canvas in
            let unit = canvas.width / 50
            for (rects, color) in [(Self.bars, Theme.border), (Self.letter, Theme.text3)] {
                var path = Path()
                for rect in rects {
                    path.addRect(CGRect(x: rect.minX * unit, y: rect.minY * unit, width: rect.width * unit, height: rect.height * unit))
                }
                context.fill(path, with: .color(color))
            }
        }
        .frame(width: size, height: size)
        .accessibilityHidden(true)
    }

    private static let bars = rects(
        "M21 8h7v1h-7zM17 9h4v1h-4zM28 9h4v1h-4zM16 10h1v1h-1zM32 10h1v1h-1zM14 11h2v1h-2zM33 11h2v1h-2z"
        + "M13 12h1v1h-1zM35 12h1v1h-1zM18 13h6v2h-6zM25 13h6v2h-6zM11 14h2v1h-2zM36 14h2v1h-2z"
        + "M10 16h1v1h-1zM13 16h4v2h-4zM18 16h6v2h-6zM25 16h6v2h-6zM32 16h4v2h-4zM38 16h1v1h-1z"
        + "M9 17h1v2h-1zM39 17h1v2h-1zM13 19h4v2h-4zM18 19h6v2h-6zM25 19h6v2h-6zM32 19h4v2h-4z"
        + "M8 20h1v1h-1zM40 20h1v1h-1z"
        + "M7 22h5v2h-5zM13 22h4v2h-4zM18 22h6v2h-6zM25 22h6v2h-6zM32 22h4v2h-4zM37 22h5v2h-5z"
        + "M7 25h5v2h-5zM13 25h4v2h-4zM18 25h6v2h-6zM25 25h6v2h-6zM32 25h4v2h-4zM37 25h5v2h-5z"
        + "M7 28h5v2h-5zM13 28h4v2h-4zM18 28h6v2h-6zM25 28h6v2h-6zM32 28h4v2h-4zM37 28h5v2h-5z"
        + "M8 31h4v1h-4zM13 31h4v1h-4zM18 31h6v1h-6zM25 31h6v1h-6zM32 31h4v1h-4zM37 31h4v1h-4z"
        + "M9 33h3v1h-3zM13 33h4v1h-4zM18 33h6v1h-6zM25 33h6v1h-6zM32 33h4v1h-4zM37 33h3v1h-3z"
        + "M10 35h2v1h-2zM13 35h4v1h-4zM18 35h6v1h-6zM25 35h6v1h-6zM32 35h4v1h-4zM37 35h2v1h-2z"
        + "M13 37h4v1h-4zM18 37h6v1h-6zM25 37h6v1h-6zM32 37h4v1h-4z"
        + "M14 39h3v1h-3zM18 39h6v1h-6zM25 39h6v1h-6zM32 39h3v1h-3zM18 41h6v1h-6zM25 41h6v1h-6z"
    )
    private static let letter = rects("M21 20h2v8h-2zM23 21h1v3.6h-1zM24 22.2h1v3.6h-1zM25 23.4h1v3.6h-1zM26 20h2v8h-2z")

    /// Reads rectangles written `Mx yhWvHh-Wz`.
    private static func rects(_ svg: String) -> [CGRect] {
        svg.split(separator: "z").compactMap { part in
            let numbers = part.replacingOccurrences(of: "M", with: "")
                .replacingOccurrences(of: "h", with: " ").replacingOccurrences(of: "v", with: " ")
                .split(separator: " ").compactMap { Double($0) }
            guard numbers.count >= 4 else { return nil }
            return CGRect(x: numbers[0], y: numbers[1], width: numbers[2], height: numbers[3])
        }
    }
}
