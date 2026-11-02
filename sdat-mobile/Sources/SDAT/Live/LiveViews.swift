import AVFoundation
import CoreImage.CIFilterBuiltins
import SwiftUI

/// Listen Together panel: start / share QR / listeners / leave.
struct LiveSheet: View {
    @State private var live = Live.shared
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List { content }
                .navigationTitle("Listen Together")
                .navigationBarTitleDisplayMode(.inline)
                .toolbar {
                    ToolbarItem(placement: .topBarTrailing) {
                        Button("Done") { live.dismissEnded(); dismiss() }
                    }
                }
        }
        .sheet(isPresented: $live.showScanner) { LiveScannerSheet() }
    }

    @ViewBuilder private var content: some View {
        switch live.mode {
        case .idle:
            Section {
                Button { Task { await live.startHosting() } } label: {
                    Label("Start a session", systemImage: "dot.radiowaves.left.and.right")
                }
                Button { live.showScanner = true } label: {
                    Label("Scan an invitation", systemImage: "qrcode.viewfinder")
                }
            } footer: {
                Text("Everyone listens with their own TIDAL account. Only the host controls playback.")
            }
        case .starting, .joining:
            HStack { ProgressView(); Text(live.mode == .starting ? "Starting…" : "Joining…").padding(.leading, 8) }
            Button("Cancel", role: .destructive) { live.leave() }
        case .hosting:
            if let invite = live.invite {
                Section {
                    QRCode(text: invite).frame(maxWidth: .infinity).padding(.vertical, 8)
                    ShareLink(item: invite) { Label("Share link", systemImage: "square.and.arrow.up") }
                } footer: { Text("Friends scan this from My Collection.") }
            }
            Section("Listening (\(live.guests.count))") {
                if live.guests.isEmpty { Text("Nobody yet").foregroundStyle(.secondary) }
                ForEach(live.guests, id: \.peer) { Text($0.name) }
            }
            Button("End session", role: .destructive) { live.leave() }
        case .joined:
            Section {
                LabeledContent("Host", value: live.hostName)
                if live.reconnecting {
                    HStack { ProgressView(); Text("Reconnecting…").padding(.leading, 8) }
                }
            }
            Section {
                Stepper("Output delay: \(live.extraLatencyMs) ms", value: $live.extraLatencyMs, in: 0...500, step: 10)
            } footer: { Text("Raise this if your speakers or headphones sound behind.") }
            if !live.upNext.isEmpty {
                Section("Up next") {
                    ForEach(live.upNext, id: \.id) { t in
                        VStack(alignment: .leading) {
                            Text(t.title)
                            Text(t.artists.map(\.name).joined(separator: ", ")).font(.caption).foregroundStyle(.secondary)
                        }
                    }
                }
            }
            Button("Leave", role: .destructive) { live.leave() }
        case .ended(let msg):
            Text(msg)
            Button("OK") { live.dismissEnded() }
        }
    }
}

struct QRCode: View {
    let text: String
    var body: some View {
        if let img = Self.image(text) {
            Image(uiImage: img).interpolation(.none).resizable().scaledToFit()
                .frame(width: 240, height: 240)
                .padding(12).background(.white, in: .rect(cornerRadius: 12))
        }
    }

    static func image(_ s: String) -> UIImage? {
        let f = CIFilter.qrCodeGenerator()
        f.message = Data(s.utf8)
        f.correctionLevel = "M"
        guard let out = f.outputImage?.transformed(by: .init(scaleX: 8, y: 8)),
              let cg = CIContext().createCGImage(out, from: out.extent) else { return nil }
        return UIImage(cgImage: cg)
    }
}

struct LiveScannerSheet: View {
    @Environment(\.dismiss) private var dismiss
    @State private var denied = false

    var body: some View {
        NavigationStack {
            Group {
                if denied {
                    ContentUnavailableView("Camera access needed", systemImage: "camera",
                        description: Text("Allow camera access for SDAT in Settings to scan invitations."))
                } else {
                    QRScanner { Live.shared.open($0) }.ignoresSafeArea()
                }
            }
            .navigationTitle("Scan invitation")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar { ToolbarItem(placement: .topBarTrailing) { Button("Cancel") { dismiss() } } }
        }
        .task {
            switch AVCaptureDevice.authorizationStatus(for: .video) {
            case .notDetermined: denied = !(await AVCaptureDevice.requestAccess(for: .video))
            case .authorized: denied = false
            default: denied = true
            }
        }
    }
}

/// Minimal AVFoundation QR reader; reports only SDAT invitations.
struct QRScanner: UIViewControllerRepresentable {
    let onCode: (String) -> Void

    func makeUIViewController(context: Context) -> Controller { Controller(onCode: onCode) }
    func updateUIViewController(_ vc: Controller, context: Context) {}

    final class Controller: UIViewController, AVCaptureMetadataOutputObjectsDelegate {
        private let session = AVCaptureSession()
        private let onCode: (String) -> Void
        private var preview: AVCaptureVideoPreviewLayer?
        private var done = false

        init(onCode: @escaping (String) -> Void) { self.onCode = onCode; super.init(nibName: nil, bundle: nil) }
        required init?(coder: NSCoder) { nil }

        override func viewDidLoad() {
            super.viewDidLoad()
            view.backgroundColor = .black
            guard let dev = AVCaptureDevice.default(for: .video),
                  let input = try? AVCaptureDeviceInput(device: dev), session.canAddInput(input) else { return }
            session.addInput(input)
            let out = AVCaptureMetadataOutput()
            guard session.canAddOutput(out) else { return }
            session.addOutput(out)
            out.setMetadataObjectsDelegate(self, queue: .main)
            out.metadataObjectTypes = [.qr]
            let p = AVCaptureVideoPreviewLayer(session: session)
            p.videoGravity = .resizeAspectFill
            view.layer.addSublayer(p)
            preview = p
            let s = session
            DispatchQueue.global(qos: .userInitiated).async { s.startRunning() }
        }

        override func viewDidLayoutSubviews() { super.viewDidLayoutSubviews(); preview?.frame = view.bounds }

        override func viewWillDisappear(_ animated: Bool) {
            super.viewWillDisappear(animated)
            let s = session
            DispatchQueue.global(qos: .userInitiated).async { s.stopRunning() }
        }

        func metadataOutput(_ output: AVCaptureMetadataOutput, didOutput objs: [AVMetadataObject], from c: AVCaptureConnection) {
            guard !done, let code = (objs.first as? AVMetadataMachineReadableCodeObject)?.stringValue,
                  liveIsInvite(text: code) else { return }
            done = true
            UINotificationFeedbackGenerator().notificationOccurred(.success)
            onCode(code)
        }
    }
}
