import SwiftUI

/// What another device plays or played last, with "Continue here". Sits above the bottom chrome.
struct HandoffBanner: View {
    static let height: CGFloat = 64
    let elsewhere: HandoffElsewhere

    var body: some View {
        let t = elsewhere.track
        HStack(spacing: 10) {
            Cover(url: t.album?.coverUrl, size: 46, radius: 8)
            VStack(alignment: .leading, spacing: 0) {
                Text(t.displayTitle).font(.subheadline.weight(.semibold)).lineLimit(1)
                Text(t.artistLine).font(.caption).foregroundStyle(.secondary).lineLimit(1)
                status.font(.caption.weight(.medium)).foregroundStyle(.blue).lineLimit(1)
            }
            Spacer(minLength: 0)
            Button("Continue here") { HandoffHub.shared.continueHere() }
                .font(.subheadline.weight(.semibold))
                .buttonStyle(.glassProminent)
        }
        .padding(.horizontal, 9)
        .frame(height: Self.height)
        .glassEffect(.regular.interactive(), in: .rect(cornerRadius: 24))
    }

    @ViewBuilder private var status: some View {
        let name = HandoffHub.name(elsewhere.device)
        if elsewhere.playing {
            Text("Playing on \(name)")
        } else if elsewhere.updatedAtMs > 0 {
            let at = Date(timeIntervalSince1970: Double(elsewhere.updatedAtMs) / 1000)
            TimelineView(.periodic(from: .now, by: 60)) { tl in
                Text("Paused on \(name) · \(Fmt.ago(at, now: tl.date))")
            }
        } else {
            Text("Paused on \(name)")
        }
    }
}

/// "Send to <device>" for each of the account's other players (player menu).
struct SendToMenu: View {
    @Environment(Player.self) private var player

    var body: some View {
        let hub = HandoffHub.shared
        if player.current != nil, !Live.shared.isGuest, !hub.devices.isEmpty {
            Divider()
            ForEach(hub.devices, id: \.id) { d in
                Button("Send to \(HandoffHub.name(d))", systemImage: "laptopcomputer.and.iphone") { hub.send(to: d) }
            }
        }
    }
}
