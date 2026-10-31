import SwiftUI

struct SettingsView: View {
    @Environment(Session.self) private var session
    @Environment(Preferences.self) private var prefs
    @Environment(Player.self) private var player
    @Environment(\.openURL) private var openURL
    @State private var confirmLogout = false
    @State private var lastFmUser = Keychain.get("lastfm.user")
    @State private var lastFmPending: LastFmAuth?
    @State private var listenBrainzToken = ""
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
                Toggle("Normalize volume", isOn: $prefs.normalizeVolume)
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

            Section("Appearance") {
                Picker("Theme", selection: $prefs.appearance) {
                    Text("System").tag(0)
                    Text("Light").tag(1)
                    Text("Dark").tag(2)
                }
            }

            Section("About") {
                LabeledContent("Version", value: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "dev")
                Link("Source code", destination: URL(string: "https://github.com/nichokas/Tonearm")!)
            }
        }
        .navigationTitle("Settings")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar(.hidden, for: .tabBar)
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
                    QualityBadge(quality: selection.wrappedValue)
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
        } else if let pending = lastFmPending {
            Button("I've approved it in Last.fm") {
                Task {
                    guard let sk = try? await lastfmFinishAuth(token: pending.token) else { return }
                    Keychain.set("lastfm.session", sk)
                    let name = (try? await lastfmUsername(sessionKey: sk)) ?? "Last.fm"
                    Keychain.set("lastfm.user", name)
                    lastFmUser = name
                    lastFmPending = nil
                }
            }
        } else {
            Button("Connect Last.fm") {
                Task {
                    guard let auth = try? await lastfmBeginAuth(), let url = URL(string: auth.url) else { return }
                    lastFmPending = auth
                    openURL(url)
                }
            }
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
