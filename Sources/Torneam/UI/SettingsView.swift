import SwiftUI
import SafariServices

struct SettingsView: View {
    @Environment(Session.self) private var session
    @Environment(Preferences.self) private var prefs
    @Environment(Player.self) private var player
    @Environment(\.openURL) private var openURL
    @State private var confirmLogout = false
    @State private var lastFmUser = Keychain.get("lastfm.user")
    @State private var lastFmPending: LastFmAuth?
    @State private var safariURL: IdentURL?
    @State private var lastFmError: String?
    @State private var listenBrainzToken = ""
    @State private var confirmErase = false
    @State private var hasListenBrainz = Keychain.get("listenbrainz.token") != nil

    var body: some View {
        @Bindable var prefs = prefs
        List {
            Section("Account") {
                Button("Sessions and devices") { openURL(URL(string: "https://account.tidal.com/")!) }
                Button("Log out", role: .destructive) { confirmLogout = true }
            }

            Section("Playback") {
                qualityRow("Streaming quality", selection: Binding(
                    get: { prefs.streamingQuality },
                    set: { prefs.streamingQuality = $0; player.reloadQuality() }
                ))
                Toggle("Preload next tracks", isOn: $prefs.preloadNext)
            }

            Section("Downloads") {
                qualityRow("Download quality", selection: $prefs.downloadQuality)
                Toggle("Download over cellular", isOn: $prefs.downloadOverCellular)
            }

            Section("Scrobbling") {
                Toggle("Last.fm", isOn: $prefs.scrobbleLastFm)
                if prefs.scrobbleLastFm { lastFmRow }
                Toggle("ListenBrainz", isOn: $prefs.scrobbleListenBrainz)
                if prefs.scrobbleListenBrainz { listenBrainzRow }
            }

            Section {
                Button("Erase all app data", role: .destructive) { confirmErase = true }
            } header: { Text("Developer") } footer: {
                Text("Signs out and deletes the TIDAL session, Last.fm and ListenBrainz tokens, settings, caches and stored files.")
            }

            Section("About") {
                LabeledContent("Version", value: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "dev")
                Link("Source code", destination: URL(string: "https://github.com/nichokas/Tonearm")!)
            }
        }
        .navigationTitle("Settings")
        .navigationBarTitleDisplayMode(.inline)
        .sheet(item: $safariURL, onDismiss: finishLastFm) { u in SafariView(url: u.url).ignoresSafeArea() }
        .confirmationDialog("Erase all app data?", isPresented: $confirmErase, titleVisibility: .visible) {
            Button("Erase everything", role: .destructive) { session.eraseAll() }
        } message: { Text("This cannot be undone.") }
        .confirmationDialog("Log out of TIDAL?", isPresented: $confirmLogout, titleVisibility: .visible) {
            Button("Log out", role: .destructive) { session.logOut() }
        }
    }

    private func qualityRow(_ title: String, selection: Binding<Quality>) -> some View {
        HStack {
            Text(title)
            Spacer()
            QualityMenu(selection: selection) {
                HStack(spacing: 4) {
                    QualityLogo(quality: selection.wrappedValue, height: 15)
                    Image(systemName: "chevron.up.chevron.down").font(.caption2)
                }
                .foregroundStyle(.secondary)
            }
        }
    }

    @ViewBuilder
    private var lastFmRow: some View {
        if let user = lastFmUser {
            LabeledContent("Connected as", value: user)
            Button("Disconnect Last.fm", role: .destructive) {
                Keychain.set("lastfm.session", nil)
                Keychain.set("lastfm.user", nil)
                lastFmUser = nil
            }
        } else {
            Button(lastFmPending == nil ? "Connect Last.fm" : "Connecting…") {
                Task {
                    do {
                        let auth = try await lastfmBeginAuth()
                        guard let url = URL(string: auth.url) else { return }
                        lastFmPending = auth
                        safariURL = IdentURL(url: url)
                    } catch { lastFmError = friendlyMessage(error) }
                }
            }
            .disabled(lastFmPending != nil)
            if let lastFmError { ErrorBanner(text: lastFmError) }
        }
    }

    /// Called when the in-app Safari sheet closes: finish the Last.fm handshake.
    private func finishLastFm() {
        guard let pending = lastFmPending else { return }
        Task {
            defer { lastFmPending = nil }
            do {
                guard let sk = try await lastfmFinishAuth(token: pending.token) else {
                    lastFmError = "Last.fm access wasn't approved."
                    return
                }
                Keychain.set("lastfm.session", sk)
                let name = (try? await lastfmUsername(sessionKey: sk)) ?? "Last.fm"
                Keychain.set("lastfm.user", name)
                lastFmUser = name
                lastFmError = nil
            } catch { lastFmError = friendlyMessage(error) }
        }
    }

    @ViewBuilder
    private var listenBrainzRow: some View {
        if hasListenBrainz {
            Button("Remove ListenBrainz token", role: .destructive) {
                Keychain.set("listenbrainz.token", nil)
                hasListenBrainz = false
            }
        } else {
            SecureField("ListenBrainz user token", text: $listenBrainzToken)
                .onSubmit {
                    let t = listenBrainzToken.trimmingCharacters(in: .whitespaces)
                    guard !t.isEmpty else { return }
                    Keychain.set("listenbrainz.token", t)
                    listenBrainzToken = ""
                    hasListenBrainz = true
                }
        }
    }
}

struct IdentURL: Identifiable { let url: URL; var id: String { url.absoluteString } }

struct SafariView: UIViewControllerRepresentable {
    let url: URL
    func makeUIViewController(context: Context) -> SFSafariViewController {
        let vc = SFSafariViewController(url: url)
        vc.preferredControlTintColor = .systemBlue
        vc.dismissButtonStyle = .done
        return vc
    }
    func updateUIViewController(_ vc: SFSafariViewController, context: Context) {}
}
