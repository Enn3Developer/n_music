import SwiftUI

/// The toasts over the bottom-right of the page area (spec §7.4): the red one for a playlist
/// the core turned down, and the queued one ("X plays next", Open queue). They can overlap, as
/// in Qt. In with a fade rising 8 points; out with an 80 ms fade.
struct ToastOverlay: View {
    private let toasts = Toasts.shared

    var body: some View {
        ZStack(alignment: .bottomTrailing) {
            if let queued = toasts.queued {
                QueuedToast(text: queued.text)
                    .id(queued.serial)
                    .transition(toastTransition)
            }
            if let failure = toasts.failure {
                FailureToast(failure: failure)
                    .id(failure.serial)
                    .transition(toastTransition)
            }
        }
        .animation(Motion.standard(Motion.move), value: toasts.queued)
        .animation(Motion.standard(Motion.move), value: toasts.failure)
    }

    private var toastTransition: AnyTransition {
        .asymmetric(
            insertion: .opacity.combined(with: .offset(y: Motion.shared.reduced ? 0 : 8)),
            removal: .opacity.animation(Motion.leaving(Motion.exit))
        )
    }
}

private struct QueuedToast: View {
    let text: String

    var body: some View {
        HStack(spacing: 12) {
            Text(text)
                .font(.system(size: 13))
                .foregroundStyle(Theme.text)
                .lineLimit(1)
                .truncationMode(.tail)
            Button(tr("open_queue")) {
                Toasts.shared.dismissQueued()
                Navigation.shared.go(PageID.queue)
            }
            .buttonStyle(FlatToastButtonStyle())
        }
        .padding(.leading, 16)
        .padding(.trailing, 5)
        .frame(height: 44)
        .frame(maxWidth: 480)
        .background(RoundedRectangle(cornerRadius: 10, style: .continuous).fill(Theme.surface))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(Theme.line2))
        .fixedSize(horizontal: true, vertical: false)
        .onHover { Toasts.shared.hoverQueued($0) }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(text)
        .onAppear { NSAccessibility.post(element: NSApp as Any, notification: .announcementRequested, userInfo: [.announcement: text, .priority: NSAccessibilityPriorityLevel.high.rawValue]) }
    }
}

private struct FailureToast: View {
    let failure: Toasts.Failure

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Image(systemName: "exclamationmark.triangle")
                .foregroundStyle(Theme.danger)
                .font(.system(size: 15, weight: .medium))
                .padding(.top, 1)
            VStack(alignment: .leading, spacing: 2) {
                Text(failure.title)
                    .font(.system(size: 13, weight: .semibold))
                    .foregroundStyle(Theme.dangerTitle)
                Text(failure.message)
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.dangerText)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 0)
            IconButton(symbol: "xmark", label: tr("dismiss"), size: 24, iconSize: 14, tint: Theme.dangerText) {
                Toasts.shared.dismissFailure()
            }
        }
        .padding(12)
        .frame(maxWidth: 420, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 10, style: .continuous).fill(Theme.dangerBg))
        .overlay(RoundedRectangle(cornerRadius: 10, style: .continuous).strokeBorder(Theme.dangerLine))
        .onAppear {
            let text = "\(failure.title). \(failure.message)"
            NSAccessibility.post(element: NSApp as Any, notification: .announcementRequested, userInfo: [.announcement: text, .priority: NSAccessibilityPriorityLevel.high.rawValue])
        }
    }
}

/// The Qt FlatButton as the toast's "Open queue": 34 points, 13 Medium.
private struct FlatToastButtonStyle: ButtonStyle {
    @State private var hovering = false

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(Theme.text)
            .padding(.horizontal, 12)
            .frame(height: 34)
            .background(RoundedRectangle(cornerRadius: 8, style: .continuous)
                .fill(configuration.isPressed ? Theme.selected : hovering ? Theme.raised : .clear))
            .onHover { hovering = $0 }
    }
}
