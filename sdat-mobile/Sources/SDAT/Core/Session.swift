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
    @ObservationIgnored private var teardown: (id: UUID, task: Task<Void, Never>)?
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
        while let pendingTeardown = teardown?.task {
            await pendingTeardown.value
            guard loginAttempt == attempt else { return }
        }
        do {
            let login = try await startDeviceLogin()
            guard loginAttempt == attempt else { login.cancel(); return }
            pending = login
            let result = try await login.wait()
            guard loginAttempt == attempt else { return }
            // Over a session: the old one detaches with its own credentials; the hub then starts the new one.
            if signedIn { HandoffHub.shared.stop() }
            clearAccountState()
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

    /// Stops playback (nothing restores on next login), clears credentials and
    /// account caches, then cancels and deletes all downloads.
    func logOut() {
        beginSignOut()
        enqueueTeardown { await Downloads.shared.removeAll() }
    }

    /// Dev: wipes Keychain, defaults, caches and files, stops playback, back to login.
    func eraseAll() {
        beginSignOut()
        enqueueTeardown { await self.eraseRest() }
    }

    private func beginSignOut() {
        cancelLogin()
        SpotifyImporter.shared.clear()
        HandoffHub.shared.signOut()
        Live.shared.signOut()
        Player.shared.stopAndClear()
        tokens.clear()
        sessionChanged()
        clearAccountState()
        signedIn = false
    }

    private func enqueueTeardown(_ work: @escaping @MainActor () async -> Void) {
        let prev = teardown?.task
        let id = UUID()
        let task = Task {
            defer { if teardown?.id == id { teardown = nil } }
            await prev?.value
            await work()
        }
        teardown = (id, task)
    }

    private func eraseRest() async {
        await Downloads.shared.removeAll()
        Keychain.wipe()
        Preferences.shared.reset()
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
        clearAccountState()
        sessionChanged()
    }

    private func clearAccountState() {
        OwnPlaylists.shared.reset()
        Favs.shared.reset()
        Profile.shared.reset()
        Recents.shared.reset()
        DiscoverCache.clear()
        Chrome.shared.paths = [:]
        Chrome.shared.tab = .discover
    }
}

extension DiscoverCache {
    static func clear() {
        let url = FileManager.default.urls(for: .cachesDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("discover.json")
        try? FileManager.default.removeItem(at: url)
    }
}
