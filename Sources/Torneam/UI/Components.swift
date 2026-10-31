import SwiftUI

struct Cover: View {
    let url: String?
    var size: CGFloat? = nil
    var radius: CGFloat = 8

    var body: some View {
        AsyncImage(url: url.flatMap(URL.init(string:))) { phase in
            if let image = phase.image {
                image.resizable().scaledToFill()
            } else {
                Rectangle().fill(.quaternary)
                    .overlay(Image(systemName: "music.note").foregroundStyle(.secondary))
            }
        }
        .frame(width: size, height: size)
        .aspectRatio(1, contentMode: .fit)
        .clipShape(.rect(cornerRadius: radius))
    }
}

struct TrackRow: View {
    let track: Track
    var showCover = true

    var body: some View {
        HStack(spacing: 12) {
            if showCover { Cover(url: track.album?.coverUrl, size: 40, radius: 4) }
            VStack(alignment: .leading, spacing: 2) {
                Text(track.displayTitle).lineLimit(1)
                HStack(spacing: 4) {
                    if track.explicit {
                        Image(systemName: "e.square.fill").font(.caption2)
                    }
                    Text(track.artistLine).lineLimit(1)
                }
                .font(.subheadline).foregroundStyle(.secondary)
            }
            Spacer(minLength: 0)
            Text(Fmt.time(Double(track.durationSecs)))
                .font(.subheadline.monospacedDigit()).foregroundStyle(.secondary)
        }
        .opacity(track.streamable ? 1 : 0.4)
        .contentShape(.rect)
    }
}

/// Plays a track list and offers per-row actions.
struct TrackList: View {
    let tracks: [Track]
    var source: String?
    var showCover = true
    @Environment(Player.self) private var player

    var body: some View {
        ForEach(Array(tracks.enumerated()), id: \.offset) { i, t in
            Button { player.play(tracks, startAt: i, from: source) } label: {
                TrackRow(track: t, showCover: showCover)
            }
            .buttonStyle(.plain)
            .contextMenu {
                Button("Play Next", systemImage: "text.insert") { player.playNext(t) }
                Button("Add to Favourites", systemImage: "star") {
                    Task { try? await setFavourite(kind: .track, id: t.id, favourite: true) }
                }
                Button("Start Radio", systemImage: "dot.radiowaves.left.and.right") {
                    Task {
                        if let pager = try? await trackRadio(trackId: t.id),
                           let list = try? await pager.nextPage() {
                            player.play([t] + list, from: "\(t.title) Radio")
                        }
                    }
                }
            }
        }
    }
}

/// Loads a value once and shows progress / errors.
struct Loader<T, Content: View>: View {
    let load: () async throws -> T
    @ViewBuilder let content: (T) -> Content
    @State private var value: T?
    @State private var error: String?

    var body: some View {
        Group {
            if let value {
                content(value)
            } else if let error {
                ContentUnavailableView("Couldn't load", systemImage: "wifi.exclamationmark", description: Text(error))
            } else {
                ProgressView().frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .task {
            guard value == nil else { return }
            do { value = try await load() } catch { self.error = error.localizedDescription }
        }
    }
}

/// Hi-Res badge for the HiFi tier.
struct QualityBadge: View {
    let quality: Quality

    var body: some View {
        HStack(spacing: 6) {
            if quality == .hiFi {
                VStack(spacing: 0) {
                    Text("Hi·Res").font(.system(size: 7, weight: .heavy))
                    Text("AUDIO").font(.system(size: 5, weight: .bold))
                }
                .foregroundStyle(.black)
                .padding(2)
                .background(Color(red: 0.85, green: 0.62, blue: 0.13), in: .rect(cornerRadius: 2))
            }
            Text(quality.label).font(.footnote.weight(.semibold))
        }
    }
}

/// Liquid Glass menu shared by the player and Settings.
struct QualityMenu<Label: View>: View {
    @Binding var selection: Quality
    @ViewBuilder let label: () -> Label

    var body: some View {
        Menu {
            Picker("Quality", selection: $selection) {
                ForEach(Quality.allCases) { q in
                    Button { } label: {
                        Text(q.label)
                        Text(q.detail)
                    }
                    .tag(q)
                }
            }
        } label: { label() }
    }
}
