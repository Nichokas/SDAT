import ImageIO
import SwiftUI
import UIKit

enum Route: Hashable {
    case album(String)
    case playlist(String)
    case artist(String)
    case settings
    case diagnostics
    case downloads
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
    private(set) var collapsed = false
    /// Animated morph progress (0 expanded ... 1 collapsed); `Morph` derives the layout from it.
    private(set) var morphT: CGFloat = 0
    /// Set by a tap on the collapsed tab circle: a list still decelerating must not collapse
    /// the bar again. Cleared once that deceleration ends or any other scroll begins.
    @ObservationIgnored var holdsOpen = false
    func setCollapsed(_ on: Bool) {
        guard on != collapsed else { return }
        collapsed = on
        withAnimation(Morph.animation) { morphT = on ? 1 : 0 }
    }
    /// Round miniplayer is being pressed (drives its scale + glass highlight).
    var miniPressed = false
    var showPlayer = false
    /// Track waiting for the "Add to playlist" picker.
    var playlistPickerTrack: Track?
    var toast: String?

    var path: [Route] { paths[tab] ?? [] }
    var inDetail: Bool { !path.isEmpty }
    var inSettings: Bool { path.last == .settings || path.last == .diagnostics }

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
    @ObservationIgnored private var generation = 0
    func ids() -> Set<String> { Set(list.map(\.id)) }
    func load(force: Bool = false) async {
        guard force || !loaded else { return }
        let gen = generation
        if let l = try? await ownPlaylists(), gen == generation { list = l; loaded = true }
    }
    func contains(_ id: String) -> Bool { list.contains { $0.id == id } }
    func reset() { generation &+= 1; list = []; loaded = false }
}

/// The user's TIDAL profile picture, cached across launches.
@MainActor @Observable
final class Profile {
    static let shared = Profile()
    private(set) var pictureURL: String? = UserDefaults.standard.string(forKey: "profilePicture320")
    private var loaded = false
    @ObservationIgnored private var generation = 0
    func load() async {
        guard !loaded else { return }
        if pictureURL == nil { pictureURL = UserDefaults.standard.string(forKey: "profilePicture320") }
        let gen = generation
        if let u = try? await profilePictureUrl(), gen == generation {
            loaded = true
            pictureURL = u
            UserDefaults.standard.set(u, forKey: "profilePicture320")
        }
    }
}

/// Circular profile photo button (generic silhouette as a fallback). Opens Settings.
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
    private(set) var tracks: [String: Bool] = [:] { didSet { onChange?() } }
    /// Lets the player keep the lock-screen like button in sync with any change.
    @ObservationIgnored var onChange: (() -> Void)?
    @ObservationIgnored private var generation = 0
    /// Unfavourited here: TIDAL's collection list can still return them for a while.
    @ObservationIgnored private var removed: Set<String> = []
    func known(_ id: String) -> Bool? { tracks[id] }
    /// Ids from the user's favourite tracks list.
    func listed(_ ids: [String]) { for i in ids where !removed.contains(i) { tracks[i] = true } }
    private func set(_ id: String, _ v: Bool) {
        tracks[id] = v
        if v { removed.remove(id) } else { removed.insert(id) }
    }
    func check(_ id: String) async {
        guard tracks[id] == nil else { return }
        let gen = generation
        if let v = try? await isFavourite(kind: .track, id: id), gen == generation { tracks[id] = v }
    }
    func toggle(_ id: String) {
        let target = !(tracks[id] ?? false)
        let gen = generation
        set(id, target)
        Task {
            do {
                try await setFavourite(kind: .track, id: id, favourite: target)
                guard gen == generation else { return }
                Chrome.shared.flash(target ? "Added to favourites" : "Removed from favourites")
            } catch {
                guard gen == generation else { return }
                set(id, !target)
                Chrome.shared.flash(friendlyMessage(error))
            }
        }
    }
    func reset() { generation &+= 1; tracks = [:]; removed = [] }
}

/// Memory-cached image loader with request coalescing (Discover canvas, covers, artwork).
/// Takes remote URLs and `file://` URLs of downloaded covers.
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

    /// `maxPixel` decodes a downsampled copy: a saved cover is the largest rendition, and
    /// keeping its full bitmap cached for a 44 pt row costs megabytes each.
    func image(_ url: String, maxPixel: Int? = nil) async -> UIImage? {
        let key = maxPixel.map { "\(url) \($0)" } ?? url
        if let i = cached(key) { return i }
        // Scoped locking (async-safe): dedupe in-flight downloads per URL.
        let task: Task<UIImage?, Never> = lock.withLock {
            if let t = tasks[key] { return t }
            let t = Task.detached(priority: .userInitiated) { [session] in
                guard let u = URL(string: url) else { return nil as UIImage? }
                let data: Data?
                if u.isFileURL { data = try? Data(contentsOf: u) } else { data = try? await session.data(from: u).0 }
                guard let data else { return nil as UIImage? }
                if let maxPixel {
                    let opts: [CFString: Any] = [
                        kCGImageSourceCreateThumbnailFromImageAlways: true,
                        kCGImageSourceCreateThumbnailWithTransform: true,
                        kCGImageSourceShouldCacheImmediately: true,
                        kCGImageSourceThumbnailMaxPixelSize: maxPixel,
                    ]
                    guard let src = CGImageSourceCreateWithData(data as CFData, nil),
                          let cg = CGImageSourceCreateThumbnailAtIndex(src, 0, opts as CFDictionary) else { return nil as UIImage? }
                    return UIImage(cgImage: cg)
                }
                guard let img = UIImage(data: data) else { return nil as UIImage? }
                return img.preparingForDisplay() ?? img
            }
            tasks[key] = t
            return t
        }
        let img = await task.value
        lock.withLock { tasks[key] = nil }
        if let img { cache.setObject(img, forKey: key as NSString) }
        return img
    }

    func prefetch(_ url: String) {
        guard cached(url) == nil else { return }
        Task { _ = await image(url) }
    }

    /// Downloads into the disk cache only, without decoding: covers for a later launch.
    func warm(_ urls: [String]) {
        for u in urls.compactMap(URL.init(string:)) {
            Task.detached(priority: .utility) { [session] in _ = try? await session.data(from: u) }
        }
    }
}

/// Profile photo, or a generic person silhouette when the account has none.
struct Avatar: View {
    let url: String?
    var body: some View {
        if let url {
            CachedImage(url: url, kind: .profile).clipShape(.circle)
        } else {
            Circle().fill(Color(white: 0.22))
                .overlay { AvatarSilhouette().fill(Color(white: 0.62)) }
                .clipShape(.circle)
        }
    }
}

/// Head and shoulders; the avatar circle cuts the shoulders off.
struct AvatarSilhouette: Shape {
    func path(in r: CGRect) -> Path {
        let w = r.width, h = r.height
        var p = Path()
        p.addEllipse(in: CGRect(x: r.minX + w * 0.32, y: r.minY + h * 0.18, width: w * 0.36, height: h * 0.36))
        p.addEllipse(in: CGRect(x: r.minX + w * 0.14, y: r.minY + h * 0.62, width: w * 0.72, height: h * 0.64))
        return p
    }
}

extension Profile {
    func reset() {
        generation &+= 1
        pictureURL = nil
        loaded = false
        UserDefaults.standard.removeObject(forKey: "profilePicture320")
    }
}

extension ImageStore {
    func clear() {
        cache.removeAllObjects()
        session.configuration.urlCache?.removeAllCachedResponses()
    }
}
