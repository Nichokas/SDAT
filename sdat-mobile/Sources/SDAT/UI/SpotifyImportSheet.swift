import SwiftUI

/// Copies a public Spotify playlist into a new TIDAL playlist.
struct SpotifyImportSheet: View {
    @Environment(\.dismiss) private var dismiss
    @State private var importer = SpotifyImporter.shared
    @State private var link = ""

    var body: some View {
        NavigationStack {
            List {
                switch importer.state {
                case .idle: form(error: nil)
                case .failed(let message): form(error: message)
                case .running(let progress):
                    Section {
                        HStack {
                            Text(label(progress))
                            Spacer()
                            ProgressView()
                        }
                        // Saving always finishes, so that a cancel never leaves half a playlist.
                        Button("Cancel", role: .destructive) { importer.cancel() }.disabled(progress?.stage == .saving)
                    }
                case .done(let report): summary(report)
                }
            }
            .navigationTitle("Import from Spotify")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { Button("Close") { dismiss() } }
        }
        .presentationDetents([.medium, .large])
    }

    @ViewBuilder
    private func form(error: String?) -> some View {
        Section {
            TextField("Spotify playlist link", text: $link)
                .keyboardType(.URL)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
                .submitLabel(.go)
                .onSubmit(start)
            Button("Import", action: start).disabled(!isSpotifyPlaylistLink(link: link))
            if let error { ErrorBanner(text: error) }
        } footer: {
            Text("The playlist must be public. In Spotify, use Share, then Copy link to playlist. SDAT creates a TIDAL playlist with the tracks TIDAL has.")
        }
    }

    @ViewBuilder
    private func summary(_ report: ImportReport) -> some View {
        Section {
            LabeledContent(report.name, value: "\(report.imported) tracks")
            Button("Open playlist") {
                dismiss()
                importer.reset()
                Chrome.shared.push(.playlist(report.playlistId))
            }
            Button("Import another") { link = ""; importer.reset() }
        } footer: {
            if report.skipped > 0 {
                Text("\(report.skipped) entries were skipped: local files, podcast episodes or tracks Spotify no longer has.")
            }
        }
        if !report.missing.isEmpty {
            Section("Not on TIDAL (\(report.missing.count))") {
                ForEach(Array(report.missing.enumerated()), id: \.offset) { Text($0.element) }
            }
        }
    }

    private func start() {
        if isSpotifyPlaylistLink(link: link) { importer.start(link: link) }
    }

    private func label(_ progress: ImportProgress?) -> String {
        let stage = switch progress?.stage {
        case nil, .reading: "Reading from Spotify"
        case .matching: "Finding tracks on TIDAL"
        case .saving: "Creating the playlist"
        }
        guard let progress, progress.total > 0 else { return "\(stage)…" }
        return "\(stage)… \(progress.done) of \(progress.total)"
    }
}
