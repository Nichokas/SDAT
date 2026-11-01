import SwiftUI

/// Onboarding / TIDAL device linking: hero logo in the upper part of the screen,
/// blue Liquid Glass sign-in button at the bottom. The flow itself lives in Session.
struct LoginView: View {
    static let logoSize: CGFloat = 220

    @Environment(Session.self) private var session
    @Environment(\.openURL) private var openURL
    @State private var appeared = false
    @State private var starting = false

    var body: some View {
        ZStack {
            Color.black.ignoresSafeArea()
            GeometryReader { g in
                ScrollView {
                    VStack(spacing: 0) {
                        // Logo centre at ~37% of the screen height.
                        Color.clear.frame(height: max(24, g.size.height * 0.37 - Self.logoSize / 2))
                        hero
                        Spacer(minLength: 24)
                    }
                    .frame(maxWidth: .infinity, minHeight: g.size.height, alignment: .top)
                }
                .scrollBounceBehavior(.basedOnSize)
            }
        }
        .safeAreaInset(edge: .bottom) { bottom }
        .preferredColorScheme(.dark)
        .onAppear { withAnimation(.spring(duration: 0.8, bounce: 0.25).delay(0.05)) { appeared = true } }
        .onChange(of: session.pending == nil) { _, none in if !none { starting = false } }
        .animation(.spring(duration: 0.45), value: session.pending == nil)
    }

    @ViewBuilder private var hero: some View {
        VStack(spacing: 24) {
            LogoTile()
                .frame(width: Self.logoSize, height: Self.logoSize)
                .scaleEffect(appeared ? 1 : 0.85)
                .offset(y: appeared ? 0 : 16)
                .opacity(appeared ? 1 : 0)
            if let login = session.pending {
                VStack(spacing: 10) {
                    Text("Enter this code on TIDAL").font(.subheadline).foregroundStyle(.secondary)
                    Text(login.userCode())
                        .font(.system(.largeTitle, design: .monospaced).bold())
                        .minimumScaleFactor(0.6).lineLimit(1)
                        .textSelection(.enabled)
                    HStack(spacing: 8) {
                        ProgressView().controlSize(.small).tint(.white)
                        Text("Waiting for approval…").font(.footnote).foregroundStyle(.secondary)
                    }
                }
                .transition(.opacity.combined(with: .scale(scale: 0.95)))
            }
            if let error = session.error {
                Label(error, systemImage: "exclamationmark.triangle.fill")
                    .font(.footnote).foregroundStyle(.orange)
                    .multilineTextAlignment(.center)
            }
        }
        .padding(.horizontal, 32)
        .foregroundStyle(.white)
    }

    private var bottom: some View {
        GlassEffectContainer(spacing: 12) {
            VStack(spacing: 12) {
                if let login = session.pending {
                    GlassWideButton(title: "Open TIDAL", systemImage: "arrow.up.right") {
                        if let url = URL(string: login.verificationUrl()) { openURL(url) }
                    }
                    Button("Cancel") { session.cancelLogin() }
                        .font(.subheadline).foregroundStyle(.secondary)
                } else {
                    GlassWideButton(title: "Sign in with TIDAL", tidalMark: true, loading: starting) {
                        starting = true
                        Task { await session.beginLogin(); starting = false }
                    }
                    .disabled(starting)
                }
            }
        }
        .padding(.horizontal, 24)
        .padding(.bottom, 12)
        .offset(y: appeared ? 0 : 40)
        .opacity(appeared ? 1 : 0)
    }
}

/// Hero: the glass-plate logo artwork (black background blends with the screen).
private struct LogoTile: View {
    var body: some View {
        Group {
            if let ui = UIImage(named: "loginlogo", in: .module, with: nil) {
                Image(uiImage: ui).resizable().interpolation(.high).scaledToFit()
            } else {
                RoundedRectangle(cornerRadius: 40, style: .continuous).fill(Color.white.opacity(0.06))
            }
        }
        .accessibilityHidden(true)
    }
}

/// Full-width interactive Liquid Glass button; shows a spinner while `loading`.
private struct GlassWideButton: View {
    let title: String
    var systemImage: String? = nil
    var tidalMark = false
    var loading = false
    let action: () -> Void
    var body: some View {
        Button(action: action) {
            ZStack {
                if loading { ProgressView().tint(.white) }
                else {
                    HStack(spacing: 8) {
                        if tidalMark { TidalMark().frame(width: 22, height: 22) }
                        Text(title)
                        if let systemImage { Image(systemName: systemImage) }
                    }
                }
            }
            .font(.headline).foregroundStyle(.white)
            .frame(maxWidth: .infinity, minHeight: 56)
            .contentShape(.capsule)
        }
        .buttonStyle(.plain)
        // Same blue as the download progress ring.
        .glassEffect(.regular.tint(.blue).interactive(), in: .capsule)
        .animation(.easeInOut(duration: 0.2), value: loading)
    }
}

/// TIDAL's flat mark (Simple Icons "tidal", 24×24 viewBox) as a Shape: crisp at any
/// size, tinted by foregroundStyle.
struct TidalMark: Shape {
    func path(in r: CGRect) -> Path {
        let k = min(r.width, r.height) / 24
        let o = CGPoint(x: r.midX - 12 * k, y: r.midY - 12 * k)
        func p(_ x: CGFloat, _ y: CGFloat) -> CGPoint { CGPoint(x: o.x + x * k, y: o.y + y * k) }
        var path = Path()
        path.addLines([p(12.012, 3.992), p(8.008, 7.996), p(4.004, 3.992), p(0, 7.996), p(4.004, 12),
                       p(8.008, 7.996), p(12.012, 12), p(8.008, 16.004), p(12.012, 20.008), p(16.016, 16.004),
                       p(12.012, 12), p(16.016, 7.996)])
        path.closeSubpath()
        path.addLines([p(16.042, 7.996), p(20.021, 4.017), p(24, 7.996), p(20.021, 11.975)])
        path.closeSubpath()
        return path
    }
}
