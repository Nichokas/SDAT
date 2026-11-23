import SwiftUI

struct DownloadsView: View {
    @Environment(Player.self) private var player
    @State private var confirmAll = false

    var body: some View {
        let dl = Downloads.shared
        let saved = dl.tracks
        let active = dl.inProgress
        let failed = dl.failures.keys.sorted()
        let cached = AudioCache.shared.tracks.filter { !dl.isDownloaded($0.track.id) }
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
                            VStack(alignment: .trailing, spacing: 2) {
                                Text(Fmt.bytes(dl.entry(t.id)?.size ?? 0)).font(.caption.monospacedDigit()).foregroundStyle(.secondary)
                                if let q = dl.entry(t.id)?.qualityText {
                                    Text(q).font(.caption2.weight(.semibold)).foregroundStyle(.secondary).lineLimit(1)
                                }
                            }
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
            if !cached.isEmpty {
                Section {
                    ForEach(Array(cached.enumerated()), id: \.element.track.id) { i, c in
                        Button { player.play(cached.map(\.track), startAt: i, from: "Downloads") } label: {
                            HStack {
                                TrackRow(track: c.track)
                                VStack(alignment: .trailing, spacing: 2) {
                                    Text(Fmt.cacheBytes(c.bytes)).font(.caption.monospacedDigit()).foregroundStyle(.secondary)
                                    Text(c.quality.badgeText).font(.caption2.weight(.semibold)).foregroundStyle(.secondary).lineLimit(1)
                                }
                            }
                        }
                        .buttonStyle(.plain)
                        .trackMenu(c.track)
                        .swipeActions {
                            Button("Remove", role: .destructive) { AudioCache.shared.remove(c.track.id) }
                        }
                        .swipeActions(edge: .leading) {
                            Button("Download") { dl.download([c.track]); Chrome.shared.flash("Downloading") }.tint(.blue)
                        }
                    }
                } header: {
                    Text("Cached · \(cached.count) tracks")
                } footer: {
                    Text("Songs you played recently. The oldest go when the cache is full.")
                }
                .listRowBackground(Color.clear)
            }
        }
        .listStyle(.plain)
        .scrollContentBackground(.hidden)
        .background(Color(.systemBackground))
        .navigationTitle("Downloads")
        .onAppear { AudioCache.shared.refresh() }
        .toolbar {
            if !saved.isEmpty {
                Menu {
                    Button("Play", systemImage: "play.fill") { player.play(saved, from: "Downloads") }
                    Button("Shuffle", systemImage: "shuffle") { player.playShuffled(saved, from: "Downloads") }
                    Button("Delete All Downloads", systemImage: "trash", role: .destructive) { confirmAll = true }
                } label: { Image(systemName: "ellipsis").foregroundStyle(.primary) }
                .tint(.primary)
            }
        }
        .confirmationDialog("Delete all downloads?", isPresented: $confirmAll, titleVisibility: .visible) {
            Button("Delete \(Fmt.bytes(dl.totalSize))", role: .destructive) { Task { await dl.removeAll() } }
        }
    }
}
