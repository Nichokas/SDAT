import SwiftUI

@main
struct SDATApp: App {
    @State private var session = Session.shared
    @State private var prefs = Preferences.shared
    @State private var boot = Boot.shared
    @State private var live = Live.shared
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
            .preferredColorScheme(.dark)
            .tint(.blue)
        }
        .onChange(of: phase) { _, p in
            if p != .active { Player.shared.persist() }
            else if session.signedIn { Downloads.shared.resumePending() }
        }
    }
}
