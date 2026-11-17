import SwiftUI
import UniformTypeIdentifiers
import UIKit

private enum LogFilter: String, CaseIterable, Identifiable {
    case all = "All"
    case warnings = "Warnings and errors"
    case errors = "Errors"
    var id: Self { self }

    func admits(_ level: DiagLevel) -> Bool {
        switch self {
        case .all: true
        case .warnings: level == .error || level == .warn
        case .errors: level == .error
        }
    }
}

struct DiagnosticsView: View {
    @State private var diag = Diagnostics.shared
    @State private var stats: DiagStats?
    @State private var events: [DiagEvent] = []
    @State private var filter = LogFilter.all
    @State private var verbose = diagVerbose()
    @State private var previous = ""
    @State private var markerText = ""
    @State private var addingMarker = false
    @State private var trace: DiagEvent?
    @State private var showPrevious = false

    private static let logLimit = 300

    var body: some View {
        List {
            overview
            network
            log
            bugHunting
            if diag.available { crashReports }
        }
        .navigationTitle("Diagnostics")
        .navigationBarTitleDisplayMode(.inline)
        .task {
            previous = diagPreviousSession()
            while !Task.isCancelled {
                refresh()
                try? await Task.sleep(for: .seconds(2))
            }
        }
        .alert("Add marker", isPresented: $addingMarker) {
            TextField("What just happened?", text: $markerText)
            Button("Add") {
                let note = markerText.trimmingCharacters(in: .whitespacesAndNewlines)
                if !note.isEmpty { diagMark(note: note); refresh() }
            }
            Button("Cancel", role: .cancel) {}
        } message: { Text("The note goes into the log and the report, so it marks where to look.") }
        .sheet(item: $trace) { e in
            TextSheet(title: e.target, text: e.backtrace ?? "") { trace = nil }
        }
        .sheet(isPresented: $showPrevious) {
            TextSheet(title: "Previous session", text: previous) { showPrevious = false }
        }
    }

    // MARK: Sections

    private var overview: some View {
        Section("Overview") {
            LabeledContent("App version", value: Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "dev")
            LabeledContent("iOS", value: UIDevice.current.systemVersion)
            LabeledContent("Device", value: Self.deviceModel)
            if let s = stats {
                LabeledContent("Uptime", value: Self.duration(ms: s.uptimeMs))
                LabeledContent("CPU time, user", value: Self.seconds(ms: s.cpuUserMs))
                LabeledContent("CPU time, system", value: Self.seconds(ms: s.cpuSystemMs))
                LabeledContent("Peak memory", value: Fmt.bytes(Int64(clamping: s.maxRssKb) * 1024))
            }
        }
    }

    private var network: some View {
        Section {
            ForEach(stats?.net ?? [], id: \.category) { n in
                VStack(alignment: .leading, spacing: 3) {
                    Text(Self.name(n.category)).font(.body.weight(.semibold))
                    Text("\(n.requests) requests, \(n.failures) failed, \(Fmt.bytes(Int64(clamping: n.bytesIn))) received")
                    Text(timing(n))
                }
                .font(.subheadline)
                .foregroundStyle(.secondary)
                .padding(.vertical, 2)
            }
        } header: { Text("Network") } footer: {
            Text("Counted by the core. Cover images load outside it, so Images can stay at zero.")
        }
    }

    private func timing(_ n: DiagNetStat) -> String {
        guard n.requests > 0 else { return "No requests yet" }
        var text = "Average \(n.totalMs / n.requests) ms, slowest \(n.maxMs) ms"
        if let status = n.lastStatus { text += ", last HTTP \(status)" }
        return text
    }

    private var log: some View {
        let shown = events.reversed().filter { filter.admits($0.level) }
        return Section {
            Picker("Show", selection: $filter) {
                ForEach(LogFilter.allCases) { Text($0.rawValue).tag($0) }
            }
            if shown.isEmpty {
                Text("Nothing logged yet").foregroundStyle(.secondary)
            }
            ForEach(shown.prefix(Self.logLimit), id: \.seq) { e in
                if e.backtrace != nil {
                    Button { trace = e } label: { LogRow(event: e, hasTrace: true) }.buttonStyle(.plain)
                } else {
                    LogRow(event: e, hasTrace: false)
                }
            }
        } header: { Text("Log") } footer: {
            if shown.count > Self.logLimit { Text("Showing the latest \(Self.logLimit) of \(shown.count). Share the report for the rest.") }
        }
    }

    private var bugHunting: some View {
        Section {
            Toggle("Verbose logging", isOn: Binding(
                get: { verbose },
                set: { verbose = $0; diagSetVerbose(on: $0) }
            ))
            Button("Add marker") { markerText = ""; addingMarker = true }
            if !previous.isEmpty {
                Button("Previous session") { showPrevious = true }
            }
            ShareLink("Share report", item: DiagReport(header: Self.reportHeader),
                      preview: SharePreview("SDAT diagnostics report"))
            if diag.sentryRunning {
                Button("Send test error") {
                    diagRecord(level: .error, target: "diagnostics", message: "Test error from Diagnostics",
                               backtrace: Thread.callStackSymbols.joined(separator: "\n"))
                    Chrome.shared.flash("Test error sent")
                }
            }
        } header: { Text("Bug hunting") } footer: {
            Text("Verbose logging adds debug detail to the log and the report. Add a marker right before you reproduce a problem.")
        }
    }

    private var crashReports: some View {
        Section {
            Toggle("Send crash reports", isOn: Binding(
                get: { diag.consent == .allowed },
                set: { diag.setConsent($0 ? .allowed : .denied) }
            ))
        } header: { Text("Crash reports") } footer: {
            Text("If SDAT crashes or hits an error, a report goes to the developer so it can be fixed. It has the error, a stack trace, recent log entries, the app version, and the device model and iOS version, but no account details.")
        }
    }

    // MARK: Data

    private func refresh() {
        stats = diagStats()
        let fresh = diagEvents(since: events.last?.seq ?? 0)
        if !fresh.isEmpty { events = Array((events + fresh).suffix(2000)) }
    }

    // MARK: Formatting

    private static var deviceModel: String {
        var info = utsname()
        uname(&info)
        return withUnsafeBytes(of: &info.machine) { raw in
            String(decoding: raw.prefix { $0 != 0 }, as: UTF8.self)
        }
    }

    private static var reportHeader: String {
        let version = Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "dev"
        return "SDAT iOS \(version)\niOS \(UIDevice.current.systemVersion)\nDevice \(deviceModel)"
    }

    private static func name(_ c: DiagNetCategory) -> String {
        switch c {
        case .tidal: "TIDAL"
        case .images: "Images"
        case .lastFm: "Last.fm"
        case .listenBrainz: "ListenBrainz"
        case .other: "Other"
        }
    }

    private static func seconds(ms: UInt64) -> String { String(format: "%.2f s", Double(ms) / 1000) }

    private static func duration(ms: UInt64) -> String {
        let s = Int(ms / 1000)
        let (h, m, sec) = (s / 3600, s % 3600 / 60, s % 60)
        if h > 0 { return "\(h) h \(m) min" }
        return m > 0 ? "\(m) min \(sec) s" : "\(sec) s"
    }
}

/// Built only when the user actually shares it: the full report formats every logged event.
private struct DiagReport: Transferable {
    let header: String

    static var transferRepresentation: some TransferRepresentation {
        DataRepresentation(exportedContentType: .plainText) { report in
            Data(diagReport(extra: report.header).utf8)
        }
    }
}

extension DiagEvent: Identifiable {
    public var id: UInt64 { seq }
}

private struct LogRow: View {
    let event: DiagEvent
    let hasTrace: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 3) {
            HStack(spacing: 6) {
                Text(label).font(.caption.weight(.bold)).foregroundStyle(color)
                Text(Date(timeIntervalSince1970: Double(event.atMs) / 1000).formatted(date: .omitted, time: .standard))
                Text(event.target).lineLimit(1)
                Spacer(minLength: 0)
                if hasTrace { Image(systemName: "chevron.right").font(.caption2) }
            }
            .font(.caption)
            .foregroundStyle(.secondary)
            Text(event.message).font(.footnote).lineLimit(4).frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.vertical, 2)
        .contentShape(.rect)
    }

    private var label: String {
        switch event.level {
        case .error: "ERROR"
        case .warn: "WARN"
        case .info: "INFO"
        case .debug: "DEBUG"
        case .trace: "TRACE"
        }
    }

    private var color: Color {
        switch event.level {
        case .error: .red
        case .warn: .orange
        default: .secondary
        }
    }
}

/// Monospaced, selectable text (a stack trace or the previous session's log).
private struct TextSheet: View {
    let title: String
    let text: String
    let close: () -> Void

    var body: some View {
        NavigationStack {
            ScrollView {
                Text(text)
                    .font(.system(.caption, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding()
            }
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .confirmationAction) { Button("Done", action: close) } }
        }
    }
}
