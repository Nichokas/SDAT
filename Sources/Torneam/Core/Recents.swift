import Foundation
import Observation

/// Recently opened albums/playlists, kept on device (tonearm-core has no recents API).
@MainActor @Observable
final class Recents {
    static let shared = Recents()

    struct Item: Codable, Hashable, Identifiable {
        enum Kind: String, Codable { case album, playlist, mix }
        var kind: Kind
        var id: String
        var title: String
        var subtitle: String
        var coverUrl: String?
    }

    private(set) var items: [Item] = []
    private let key = "recents"

    private init() {
        if let data = UserDefaults.standard.data(forKey: key),
           let decoded = try? JSONDecoder().decode([Item].self, from: data) {
            items = decoded
        }
    }

    func add(_ item: Item) {
        items.removeAll { $0.id == item.id && $0.kind == item.kind }
        items.insert(item, at: 0)
        items = Array(items.prefix(100))
        if let data = try? JSONEncoder().encode(items) {
            UserDefaults.standard.set(data, forKey: key)
        }
    }
}
