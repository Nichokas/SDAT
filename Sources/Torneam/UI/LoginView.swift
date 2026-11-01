import SwiftUI

/// TIDAL device linking: show the code, open link.tidal.com, wait for approval.
struct LoginView: View {
    @Environment(Session.self) private var session
    @Environment(\.openURL) private var openURL

    var body: some View {
        VStack(spacing: 24) {
            Spacer()
            Image(systemName: "waveform.circle.fill").font(.system(size: 72)).foregroundStyle(.tint)
            Text("SDAT").font(.largeTitle.bold())
            if let login = session.pending {
                Text("Enter this code on TIDAL").foregroundStyle(.secondary)
                Text(login.userCode()).font(.system(.largeTitle, design: .monospaced).bold())
                    .textSelection(.enabled)
                Button("Open TIDAL") {
                    if let url = URL(string: login.verificationUrl()) { openURL(url) }
                }
                .buttonStyle(.glassProminent)
                ProgressView("Waiting for approval…")
                Button("Cancel", role: .cancel) { session.cancelLogin() }
            } else {
                Button("Log in with TIDAL") { Task { await session.beginLogin() } }
                    .buttonStyle(.glassProminent)
                    .controlSize(.large)
            }
            if let error = session.error {
                Text(error).font(.footnote).foregroundStyle(.red).multilineTextAlignment(.center)
            }
            Spacer()
        }
        .padding(32)
    }
}
