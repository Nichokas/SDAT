import Foundation

/// One Spotify playlist import at a time; it keeps running while its sheet is closed.
@MainActor @Observable
final class SpotifyImporter {
    enum State {
        case idle
        /// `nil` until the core reports its first step.
        case running(ImportProgress?)
        case done(ImportReport)
        case failed(String)
    }

    static let shared = SpotifyImporter()
    private(set) var state = State.idle
    private var running: SpotifyImport?

    func start(link: String) {
        if case .running = state { return }
        let job = spotifyImport(link: link)
        running = job
        state = .running(nil)
        Task {
            let result: State
            do {
                let report = try await job.run(listener: ImportBridge(job: job))
                await OwnPlaylists.shared.load(force: true)
                result = .done(report)
            } catch FfiError.Cancelled {
                result = .idle
            } catch FfiError.Other(let detail) {
                // The core words the import's own failures (a private playlist, nothing found) for the user.
                result = .failed(detail)
            } catch {
                result = .failed(friendlyMessage(error))
            }
            running = nil
            state = result
        }
    }

    func cancel() { running?.cancel() }

    /// Forgets a finished import, so the sheet asks for a link again.
    func reset() {
        if case .running = state { return }
        state = .idle
    }

    fileprivate func progressed(_ progress: ImportProgress, job: SpotifyImport) {
        if running === job { state = .running(progress) }
    }
}

/// Callbacks arrive on core threads; hop to the main actor.
private final class ImportBridge: ImportListener, @unchecked Sendable {
    private let job: SpotifyImport
    init(job: SpotifyImport) { self.job = job }
    func onProgress(progress: ImportProgress) {
        let job = job
        Task { @MainActor in SpotifyImporter.shared.progressed(progress, job: job) }
    }
}
