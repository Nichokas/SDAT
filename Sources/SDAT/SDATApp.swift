import SwiftUI

@main
struct SDATApp: App {
    @State private var session = Session.shared
    @State private var prefs = Preferences.shared
    @State private var boot = Boot.shared
    @Environment(\.scenePhase) private var phase

    var body: some Scene {
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
            .preferredColorScheme(.dark)
            .tint(.blue)
        }
        .onChange(of: phase) { _, p in
            if p != .active { Player.shared.persist() }
            else if session.signedIn { Downloads.shared.resumePending() }
        }
    }
}
