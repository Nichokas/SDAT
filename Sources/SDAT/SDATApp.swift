import SwiftUI

@main
struct SDATApp: App {
    @State private var session = Session.shared
    @State private var prefs = Preferences.shared
    @Environment(\.scenePhase) private var phase

    var body: some Scene {
        WindowGroup {
            Group {
                if session.signedIn {
                    // RootView loads underneath, so the loader lasts only as long as the real load.
                    ZStack {
                        RootView()
                        if session.booting {
                            BootLoaderView().transition(.opacity).zIndex(1)
                        }
                    }
                    .animation(.easeInOut(duration: 0.45), value: session.booting)
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
        .onChange(of: phase) { _, p in if p != .active { Player.shared.persist() } }
    }
}
