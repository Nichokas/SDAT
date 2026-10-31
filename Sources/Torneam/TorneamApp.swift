import SwiftUI

@main
struct TorneamApp: App {
    @State private var session = Session.shared
    @State private var prefs = Preferences.shared

    var body: some Scene {
        WindowGroup {
            Group {
                if session.signedIn {
                    RootView()
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
    }
}
