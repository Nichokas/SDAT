import Foundation
import Security

/// Minimal generic-password Keychain wrapper.
enum Keychain {
    private static let service = "com.nichokas.SDAT"

    static func get(_ key: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var out: AnyObject?
        guard SecItemCopyMatching(query as CFDictionary, &out) == errSecSuccess,
              let data = out as? Data else { return nil }
        return String(data: data, encoding: .utf8)
    }

    /// Deletes every item this app stored.
    static func wipe() {
        SecItemDelete([kSecClass as String: kSecClassGenericPassword, kSecAttrService as String: service] as CFDictionary)
    }

    static func set(_ key: String, _ value: String?) {
        let base: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
        ]
        SecItemDelete(base as CFDictionary)
        guard let value else { return }
        var add = base
        add[kSecValueData as String] = Data(value.utf8)
        add[kSecAttrAccessible as String] = kSecAttrAccessibleAfterFirstUnlock
        SecItemAdd(add as CFDictionary, nil)
    }
}

/// TIDAL session kept in the Keychain; read by tonearm-core through the TokenStore callback.
final class KeychainTokenStore: TokenStore, @unchecked Sendable {
    func refreshToken() -> String? { Keychain.get("tidal.refresh_token") }
    func userId() -> String? { Keychain.get("tidal.user_id") }

    func save(_ tokens: LoginTokens) {
        Keychain.set("tidal.refresh_token", tokens.refreshToken)
        Keychain.set("tidal.user_id", tokens.userId)
    }

    func clear() {
        Keychain.set("tidal.refresh_token", nil)
        Keychain.set("tidal.user_id", nil)
    }
}
