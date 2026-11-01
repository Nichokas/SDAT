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
        error = nil
        do {
            let login = try await startDeviceLogin()
            pending = login
            let result = try await login.wait()
            tokens.save(result)
            sessionChanged()
            signedIn = true
        } catch {
            self.error = friendlyMessage(error)
        }
        pending = nil
    }

    func cancelLogin() {
        pending?.cancel()
        pending = nil
    }

    /// Stops playback (nothing restores on next login), cancels and deletes all
    /// downloads, then clears the credentials.
    func logOut() {
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
        Player.shared.stopAndClear()
        Task { await eraseRest() }
    }

    private func eraseRest() async {
        await Downloads.shared.removeAll()
        Keychain.wipe()
        if let id = Bundle.main.bundleIdentifier { UserDefaults.standard.removePersistentDomain(forName: id) }
        URLCache.shared.removeAllCachedResponses()
        ImageStore.shared.clear()
        let fm = FileManager.default
        for dir in [FileManager.SearchPathDirectory.cachesDirectory, .applicationSupportDirectory, .documentDirectory] {
            for base in fm.urls(for: dir, in: .userDomainMask) {
                for f in (try? fm.contentsOfDirectory(at: base, includingPropertiesForKeys: nil)) ?? [] { try? fm.removeItem(at: f) }
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
