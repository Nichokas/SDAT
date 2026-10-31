import SwiftUI

/// Bottom accessory. Expanded: cover, title and play button. When the tab bar minimises
/// it becomes a compact capsule. The play button's ring shows progress: blue clockwise
/// from 12 o'clock, the remainder dashed white at 40%.
struct MiniPlayer: View {
    @Environment(Player.self) private var player
    @Environment(\.tabViewBottomAccessoryPlacement) private var placement
    let open: () -> Void

    var body: some View {
        HStack(spacing: 10) {
            Cover(url: player.current?.album?.coverUrl, size: 30, radius: 6)
            if placement != .inline {
                VStack(alignment: .leading, spacing: 0) {
                    Text(player.current?.displayTitle ?? "").font(.subheadline.weight(.semibold)).lineLimit(1)
                    Text(player.current?.artistLine ?? "").font(.caption).foregroundStyle(.secondary).lineLimit(1)
                }
            } else {
                Text(player.current?.displayTitle ?? "").font(.footnote.weight(.semibold)).lineLimit(1)
            }
            Spacer(minLength: 0)
            Button { player.toggle() } label: {
                Image(systemName: player.isPlaying ? "pause.fill" : "play.fill")
                    .frame(width: 32, height: 32)
                    .overlay { ProgressRing(progress: player.progress) }
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 12)
        .contentShape(.rect)
        .onTapGesture(perform: open)
    }
}

struct ProgressRing: View {
    let progress: Double

    var body: some View {
        ZStack {
            Circle()
                .trim(from: progress, to: 1)
                .stroke(.white.opacity(0.4), style: StrokeStyle(lineWidth: 2, dash: [3, 3]))
            Circle()
                .trim(from: 0, to: progress)
                .stroke(.blue, style: StrokeStyle(lineWidth: 2.5, lineCap: .round))
        }
        .rotationEffect(.degrees(-90))
        .padding(-2)
        .animation(.linear(duration: 0.5), value: progress)
    }
}
