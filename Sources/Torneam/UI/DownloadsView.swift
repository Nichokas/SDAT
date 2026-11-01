import SwiftUI

/// Offline tracks: in-progress, failed (retry), saved (swipe to delete), total size.
struct DownloadsView: View {
    @Environment(Player.self) private var player
    @State private var confirmAll = false

    var body: some View {
        let dl = Downloads.shared
        let saved = dl.tracks
        let active = dl.inProgress
        let failed = dl.failures.keys.sorted()
        List {
            if !active.isEmpty {
                Section {
                    ForEach(active, id: \.id) { t in
                        HStack {
                            MediaRow(title: t.displayTitle, subtitle: t.artistLine, cover: t.album?.coverUrl)
                            Spacer()
                            DownloadBadge(id: t.id)
                        }
                        .swipeActions { Button("Cancel", role: .destructive) { dl.cancel(t.id) } }
                    }
                } header: {
                    HStack { Text("Downloading"); Spacer(); Button("Cancel all") { dl.cancelAll() }.font(.footnote) }
                }
            }
            if !failed.isEmpty {
                Section("Failed") {
                    ForEach(failed, id: \.self) { id in
                        Text(dl.failures[id] ?? "").font(.footnote).foregroundStyle(.orange)
                    }
                }
            }
            Section {
                if saved.isEmpty {
                    Text("Downloaded tracks play without a connection. Use “Download” in a track, album or playlist menu.")
                        .font(.footnote).foregroundStyle(.secondary)
                }
                ForEach(Array(saved.enumerated()), id: \.element.id) { i, t in
                    Button { player.play(saved, startAt: i, from: "Downloads") } label: {
                        HStack {
                            TrackRow(track: t)
                            Text(Fmt.bytes(dl.entry(t.id)?.size ?? 0)).font(.caption.monospacedDigit()).foregroundStyle(.secondary)
                        }
                    }
                    .buttonStyle(.plain)
                    .trackMenu(t)
                    .swipeActions { Button("Delete", role: .destructive) { dl.remove(t.id) } }
                }
            } header: {
                if !saved.isEmpty { Text("\(saved.count) tracks · \(Fmt.bytes(dl.totalSize))") }
            }
            .listRowBackground(Color.clear)
        }
        .listStyle(.plain)
        .scrollContentBackground(.hidden)
        .background(Color.black)
        .navigationTitle("Downloads")
        .toolbar {
            if !saved.isEmpty {
                Menu {
                    Button("Play", systemImage: "play.fill") { player.play(saved, from: "Downloads") }
                    Button("Shuffle", systemImage: "shuffle") { player.playShuffled(saved, from: "Downloads") }
                    Button("Delete All Downloads", systemImage: "trash", role: .destructive) { confirmAll = true }
                } label: { Image(systemName: "ellipsis").foregroundStyle(.white) }
                .tint(.white)
            }
        }
        .confirmationDialog("Delete all downloads?", isPresented: $confirmAll, titleVisibility: .visible) {
            Button("Delete \(Fmt.bytes(dl.totalSize))", role: .destructive) { dl.removeAll() }
        }
    }
}
