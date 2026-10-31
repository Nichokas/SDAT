import SwiftUI
import UIKit

enum Route: Hashable {
    case album(String)
    case playlist(String)
    case artist(String)
    case settings
    case collectionTracks, collectionAlbums, collectionArtists, collectionPlaylists
    case explore(path: String, title: String)
}

enum AppTab: Hashable { case discover, search, collection }

/// Navigation + bottom-chrome state shared by the tabs, the custom tab bar and the miniplayer.
@MainActor @Observable
final class Chrome {
    static let shared = Chrome()

    var tab: AppTab = .discover
    var paths: [AppTab: [Route]] = [.discover: [], .search: [], .collection: []]
    /// Root list scrolled down: tab bar shrinks to a circle, miniplayer grows to a capsule.
    var collapsed = false
    var showPlayer = false
    /// Track waiting for the "Add to playlist" picker.
    var playlistPickerTrack: Track?
    var toast: String?

    var path: [Route] { paths[tab] ?? [] }
    var inDetail: Bool { !path.isEmpty }
    var inSettings: Bool { path.last == .settings }

    func binding(_ tab: AppTab) -> Binding<[Route]> {
        Binding(get: { self.paths[tab] ?? [] }, set: { self.paths[tab] = $0 })
    }

    func push(_ r: Route) { paths[tab, default: []].append(r) }

    func flash(_ text: String) {
        toast = text
        Task { try? await Task.sleep(for: .seconds(1.6)); if toast == text { toast = nil } }
    }
}

/// The user's own playlists (for "Add to playlist" and to hide the star on them).
@MainActor @Observable
final class OwnPlaylists {
    static let shared = OwnPlaylists()
    private(set) var list: [OwnPlaylist] = []
    private var loaded = false
    func ids() -> Set<String> { Set(list.map(\.id)) }
    func load(force: Bool = false) async {
        guard force || !loaded else { return }
        if let l = try? await ownPlaylists() { list = l; loaded = true }
    }
    func contains(_ id: String) -> Bool { list.contains { $0.id == id } }
}

/// The user's TIDAL profile picture, cached across launches.
@MainActor @Observable
final class Profile {
    static let shared = Profile()
    private(set) var pictureURL: String? = UserDefaults.standard.string(forKey: "profilePicture320")
    private var loaded = false
    func load() async {
        guard !loaded else { return }
        if pictureURL == nil { pictureURL = UserDefaults.standard.string(forKey: "profilePicture320") }
        if let u = try? await profilePictureUrl() {
            loaded = true
            pictureURL = u
            UserDefaults.standard.set(u, forKey: "profilePicture320")
        }
    }
}

/// Circular profile photo button (generic icon only as a fallback). Opens Settings.
struct ProfileButton: View {
    var size: CGFloat = 44
    @State private var profile = Profile.shared
    var body: some View {
        Button { Chrome.shared.push(.settings) } label: {
            Group {
                Avatar(url: profile.pictureURL).padding(4)
            }
            .frame(width: size, height: size)
        }
        .buttonStyle(.plain)
        .glassEffect(.regular.interactive(), in: .circle)
        .task { await profile.load() }
    }
}

/// Cached favourite state for tracks, so context menus reflect it.
@MainActor @Observable
final class Favs {
    static let shared = Favs()
    private(set) var tracks: [String: Bool] = [:]
    func known(_ id: String) -> Bool? { tracks[id] }
    func mark(_ ids: [String], _ v: Bool) { for i in ids { tracks[i] = v } }
    func check(_ id: String) async {
        guard tracks[id] == nil else { return }
        if let v = try? await isFavourite(kind: .track, id: id) { tracks[id] = v }
    }
    func toggle(_ id: String) {
        let target = !(tracks[id] ?? false)
        tracks[id] = target
        Task {
            do {
                try await setFavourite(kind: .track, id: id, favourite: target)
                Chrome.shared.flash(target ? "Added to favourites" : "Removed from favourites")
            } catch { tracks[id] = !target; Chrome.shared.flash(error.localizedDescription) }
        }
    }
}

/// Memory-cached image loader with request coalescing (Discover canvas, covers, artwork).
final class ImageStore: @unchecked Sendable {
    static let shared = ImageStore()
    private let cache = NSCache<NSString, UIImage>()
    private var tasks: [String: Task<UIImage?, Never>] = [:]
    private let lock = NSLock()
    private let session: URLSession = {
        let c = URLSessionConfiguration.default
        c.urlCache = URLCache(memoryCapacity: 32 << 20, diskCapacity: 256 << 20)
        c.requestCachePolicy = .returnCacheDataElseLoad
        c.httpMaximumConnectionsPerHost = 8
        return URLSession(configuration: c)
    }()

    init() { cache.countLimit = 600 }

    func cached(_ url: String) -> UIImage? { cache.object(forKey: url as NSString) }

    func image(_ url: String) async -> UIImage? {
        if let i = cached(url) { return i }
        lock.lock()
        let task: Task<UIImage?, Never>
        if let t = tasks[url] { task = t } else {
            task = Task.detached(priority: .userInitiated) { [session] in
                guard let u = URL(string: url), let (data, _) = try? await session.data(from: u),
                      let img = UIImage(data: data) else { return nil }
                return img.preparingForDisplay() ?? img
            }
            tasks[url] = task
        }
        lock.unlock()
        let img = await task.value
        lock.lock(); tasks[url] = nil; lock.unlock()
        if let img { cache.setObject(img, forKey: url as NSString) }
        return img
    }

    func prefetch(_ url: String) {
        guard cached(url) == nil else { return }
        Task { _ = await image(url) }
    }
}

/// Profile photo, or a default avatar in the TIDAL style (a looped squiggle on a dark circle).
struct Avatar: View {
    let url: String?
    var body: some View {
        if let url {
            CachedImage(url: url).clipShape(.circle)
        } else {
            Circle().fill(Color(white: 0.1))
                .overlay {
                    GeometryReader { g in
                        let w = g.size.width
                        AvatarGlyph()
                            .stroke(LinearGradient(colors: [Color(red: 0.55, green: 0.7, blue: 1), .white], startPoint: .topLeading, endPoint: .bottomTrailing),
                                    style: StrokeStyle(lineWidth: max(1.5, w * 0.06), lineCap: .round, lineJoin: .round))
                            .padding(w * 0.24)
                    }
                }
        }
    }
}

/// Two crossing loops, like a pretzel/infinity squiggle.
struct AvatarGlyph: Shape {
    func path(in r: CGRect) -> Path {
        let w = r.width, h = r.height, x = r.minX, y = r.minY
        var p = Path()
        p.move(to: CGPoint(x: x + w * 0.15, y: y + h * 0.95))
        p.addCurve(to: CGPoint(x: x + w * 0.42, y: y + h * 0.15),
                   control1: CGPoint(x: x + w * 0.45, y: y + h * 0.75), control2: CGPoint(x: x + w * 0.62, y: y + h * 0.2))
        p.addCurve(to: CGPoint(x: x + w * 0.2, y: y + h * 0.45),
                   control1: CGPoint(x: x + w * 0.25, y: y + h * 0.1), control2: CGPoint(x: x + w * 0.05, y: y + h * 0.3))
        p.addCurve(to: CGPoint(x: x + w * 0.8, y: y + h * 0.4),
                   control1: CGPoint(x: x + w * 0.4, y: y + h * 0.65), control2: CGPoint(x: x + w * 0.95, y: y + h * 0.6))
        p.addCurve(to: CGPoint(x: x + w * 0.58, y: y + h * 0.15),
                   control1: CGPoint(x: x + w * 0.7, y: y + h * 0.18), control2: CGPoint(x: x + w * 0.62, y: y + h * 0.12))
        p.addCurve(to: CGPoint(x: x + w * 0.88, y: y + h * 0.95),
                   control1: CGPoint(x: x + w * 0.5, y: y + h * 0.55), control2: CGPoint(x: x + w * 0.65, y: y + h * 0.8))
        return p
    }
}

extension Profile {
    func reset() { pictureURL = nil; loaded = false }
}

extension ImageStore {
    func clear() {
        cache.removeAllObjects()
        session.configuration.urlCache?.removeAllCachedResponses()
    }
}
