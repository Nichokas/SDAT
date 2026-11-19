import Foundation
import Observation

/// Login state and the device-link flow.
@MainActor @Observable
final class Session {
    static let shared = Session()

    let tokens = KeychainTokenStore()
    private(set) var signedIn = false
    private(set) var pending: DeviceLogin?
    var error: String?
    private var loginAttempt: UUID?
    var startingLogin: Bool { loginAttempt != nil && pending == nil }
    private init() {
        try? FileManager.default.createDirectory(
            at: FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0],
            withIntermediateDirectories: true
        )
        initialize(tokens: tokens, config: DefaultsAppConfig())
        signedIn = tokens.refreshToken() != nil
    }

    /// Starts TIDAL device linking; the user approves the code on link.tidal.com.
    func beginLogin() async {
        cancelLogin()
        let attempt = UUID()
        loginAttempt = attempt
        error = nil
        do {
            let login = try await startDeviceLogin()
            guard loginAttempt == attempt else { login.cancel(); return }
            pending = login
            let result = try await login.wait()
            guard loginAttempt == attempt else { return }
            // Over a session: the old one detaches with its own credentials; the hub then starts the new one.
            if signedIn { HandoffHub.shared.stop() }
            tokens.save(result)
            HandoffHub.shared.signedIn()
            sessionChanged()
            Boot.shared.start()   // first-time warm-up, only after a fresh sign-in
            signedIn = true
        } catch {
            guard loginAttempt == attempt else { return }
            self.error = friendlyMessage(error)
        }
        loginAttempt = nil
        pending = nil
    }

    func cancelLogin() {
        loginAttempt = nil
        pending?.cancel()
        pending = nil
    }

    /// Stops playback (nothing restores on next login), cancels and deletes all
    /// downloads, then clears the credentials.
    func logOut() {
        SpotifyImporter.shared.clear()
        HandoffHub.shared.signOut()
        Live.shared.signOut()
        Player.shared.stopAndClear()
        Task {
            await Downloads.shared.removeAll()
            tokens.clear()
            sessionChanged()
            signedIn = false
        }
    }

    /// Dev: wipes Keychain, defaults, caches and files, stops playback, back to login.
    func eraseAll() {
        SpotifyImporter.shared.clear()
        HandoffHub.shared.signOut()
        Live.shared.signOut()
        Player.shared.stopAndClear()
        Task { await eraseRest() }
    }

    private func eraseRest() async {
        await Downloads.shared.removeAll()
        Keychain.wipe()
        Preferences.shared.reset()
        Recents.shared.reset()
        if let id = Bundle.main.bundleIdentifier { UserDefaults.standard.removePersistentDomain(forName: id) }
        Diagnostics.shared.setConsent(.unset)
        // Empties the diagnostics log and the previous session; the core keeps diag.log open, so its directory stays.
        diagClear()
        URLCache.shared.removeAllCachedResponses()
        ImageStore.shared.clear()
        let fm = FileManager.default
        for dir in [FileManager.SearchPathDirectory.cachesDirectory, .applicationSupportDirectory, .documentDirectory] {
            for base in fm.urls(for: dir, in: .userDomainMask) {
                for f in (try? fm.contentsOfDirectory(at: base, includingPropertiesForKeys: nil)) ?? []
                where !(dir == .applicationSupportDirectory && f.lastPathComponent == "diag") {
                    try? fm.removeItem(at: f)
                }
            }
        }
        for f in (try? fm.contentsOfDirectory(at: fm.temporaryDirectory, includingPropertiesForKeys: nil)) ?? [] { try? fm.removeItem(at: f) }
        try? fm.createDirectory(at: fm.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0], withIntermediateDirectories: true)
        Profile.shared.reset()
        Chrome.shared.paths = [:]
        Chrome.shared.tab = .discover
        sessionChanged()
        signedIn = false
    }
}
