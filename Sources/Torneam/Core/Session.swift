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
            self.error = error.localizedDescription
        }
        pending = nil
    }

    func cancelLogin() {
        pending?.cancel()
        pending = nil
    }

    func logOut() {
        tokens.clear()
        sessionChanged()
        signedIn = false
    }
}
