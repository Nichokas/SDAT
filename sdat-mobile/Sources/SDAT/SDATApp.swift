import SwiftUI

@main
struct SDATApp: App {
    // First, so the diagnostics log starts before the other singletons do any work.
    @State private var diagnostics = Diagnostics.shared
    @State private var session = Session.shared
    @State private var prefs = Preferences.shared
    @State private var boot = Boot.shared
    @State private var live = Live.shared
    @State private var handoff = HandoffHub.shared
    @Environment(\.scenePhase) private var phase

    var body: some Scene {
        // Read in body so Observation re-renders the scene when either flag changes.
        let showLive = live.showSheet && !Chrome.shared.showPlayer
        WindowGroup {
            Group {
                if session.signedIn {
                    // Fresh sign-in: warm caches first, then the app (fade).
                    ZStack {
                        if boot.active {
                            BootLoaderView(progress: boot.progress).transition(.opacity)
                        } else {
                            RootView().transition(.opacity)
                        }
                    }
                    .animation(.easeInOut(duration: 0.45), value: boot.active)
                } else {
                    LoginView()
                }
            }
            .environment(session)
            .environment(Player.shared)
            .environment(prefs)
            .environment(Recents.shared)
            .environment(Chrome.shared)
            // A root sheet can't present while the player sheet is up; PlayerSheet presents
            // LiveSheet itself then. Exactly one of the two bindings is true at a time.
            .sheet(isPresented: Binding(get: { showLive },
                                        set: { if !$0 { live.showSheet = false } })) { LiveSheet() }
            .onOpenURL { url in if session.signedIn { live.open(url.absoluteString) } }
            // On the root, so a fresh install is asked before sign-in too.
            .alert("Send crash reports?", isPresented: Binding(get: { diagnostics.needsPrompt }, set: { _ in })) {
                Button("Send reports") { diagnostics.setConsent(.allowed) }
                Button("Don't send") { diagnostics.setConsent(.denied) }
            } message: {
                Text("If SDAT crashes or hits an error, send a report to the developer so it can be fixed. Reports include the error, a stack trace, recent log entries, the app version, and the device model and iOS version, but no account details. You can change this in Settings > Diagnostics.")
            }
            .tint(.blue)
        }
        .onChange(of: phase) { _, p in
            handoff.foreground = p != .background
            if p != .active { Player.shared.persist() }
            else if session.signedIn { Downloads.shared.resumePending() }
        }
    }
}
