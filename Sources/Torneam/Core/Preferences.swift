import Foundation
import SwiftUI

extension Quality: CaseIterable, Identifiable {
    public static var allCases: [Quality] { [.hiFi, .cd, .aac] }
    public var id: String { key }

    var key: String {
        switch self {
        case .hiFi: "hifi"
        case .cd: "cd"
        case .aac: "aac"
        }
    }

    init?(key: String) {
        guard let q = Quality.allCases.first(where: { $0.key == key }) else { return nil }
        self = q
    }

    var label: String {
        switch self {
        case .hiFi: "HiFi"
        case .cd: "CD"
        case .aac: "AAC"
        }
    }

    var detail: String {
        switch self {
        case .hiFi: "Up to 24-bit/192 kHz FLAC"
        case .cd: "16-bit/44.1 kHz FLAC"
        case .aac: "Up to 320 kbps AAC"
        }
    }
}

/// User preferences (UserDefaults). Secrets never go here; see Keychain.
@Observable
final class Preferences {
    static let shared = Preferences()
    private let d = UserDefaults.standard

    var streamingQuality: Quality {
        didSet { d.set(streamingQuality.key, forKey: "streamingQuality") }
    }
    var downloadQuality: Quality {
        didSet { d.set(downloadQuality.key, forKey: "downloadQuality") }
    }
    var downloadOverCellular: Bool { didSet { d.set(downloadOverCellular, forKey: "downloadOverCellular") } }
    var normalizeVolume: Bool { didSet { d.set(normalizeVolume, forKey: "normalizeVolume") } }
    var scrobbleLastFm: Bool { didSet { d.set(scrobbleLastFm, forKey: "scrobbleLastFm") } }
    var scrobbleListenBrainz: Bool { didSet { d.set(scrobbleListenBrainz, forKey: "scrobbleListenBrainz") } }
    var appearance: Int { didSet { d.set(appearance, forKey: "appearance") } } // 0 system, 1 light, 2 dark

    var colorScheme: ColorScheme? { appearance == 1 ? .light : appearance == 2 ? .dark : nil }

    private init() {
        streamingQuality = Quality(key: d.string(forKey: "streamingQuality") ?? "") ?? .hiFi
        downloadQuality = Quality(key: d.string(forKey: "downloadQuality") ?? "") ?? .hiFi
        downloadOverCellular = d.bool(forKey: "downloadOverCellular")
        normalizeVolume = d.bool(forKey: "normalizeVolume")
        scrobbleLastFm = d.bool(forKey: "scrobbleLastFm")
        scrobbleListenBrainz = d.bool(forKey: "scrobbleListenBrainz")
        appearance = d.integer(forKey: "appearance")
    }
}

/// tonearm-core reads its config through this callback.
final class DefaultsAppConfig: AppConfig, @unchecked Sendable {
    func streamingQuality() -> Quality {
        Quality(key: UserDefaults.standard.string(forKey: "streamingQuality") ?? "") ?? .hiFi
    }

    func configDir() -> String? {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first?.path
    }
}
