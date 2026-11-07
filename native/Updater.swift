// A separate native process keeps network, verification and installation off Rune's UI thread.
import AppKit
import CryptoKit
import Darwin
import Foundation

struct UpdateVersion: Comparable, Equatable {
  let numbers: [Int]
  let prerelease: [String]
  init(_ text: String) throws {
    let halves = text.split(separator: "-", maxSplits: 1, omittingEmptySubsequences: false)
    let parts = halves[0].split(separator: ".", omittingEmptySubsequences: false)
    guard parts.count == 3,
      parts.allSatisfy({
        !$0.isEmpty && $0.allSatisfy(\.isNumber) && ($0 == "0" || !$0.hasPrefix("0"))
      }),
      parts.allSatisfy({ Int($0) != nil })
    else { throw UpdateError.invalidVersion }
    numbers = parts.map { Int($0)! }
    prerelease =
      halves.count == 2
      ? halves[1].split(separator: ".", omittingEmptySubsequences: false).map(String.init) : []
    guard halves.count == 1 || !prerelease.isEmpty,
      prerelease.allSatisfy({
        !$0.isEmpty
          && $0.utf8.allSatisfy {
            $0 == 45 || (48...57).contains($0) || (65...90).contains($0) || (97...122).contains($0)
          }
      })
    else { throw UpdateError.invalidVersion }
  }
  static func < (a: Self, b: Self) -> Bool {
    if a.numbers != b.numbers { return a.numbers.lexicographicallyPrecedes(b.numbers) }
    if a.prerelease.isEmpty { return false }
    if b.prerelease.isEmpty { return true }
    for (left, right) in zip(a.prerelease, b.prerelease) where left != right {
      if let l = Int(left), let r = Int(right) { return l < r }
      if Int(left) != nil { return true }
      if Int(right) != nil { return false }
      return left < right
    }
    return a.prerelease.count < b.prerelease.count
  }
}

enum UpdateError: Error, LocalizedError {
  case invalidVersion, invalidSignature, invalidMetadata, untrustedDownload, unsupportedInstall,
    download, process
  var errorDescription: String? {
    switch self {
    case .invalidVersion: return "The release version is invalid. No update was installed."
    case .invalidSignature:
      return
        "Publisher verification failed or is not configured. No update was downloaded or installed."
    case .invalidMetadata:
      return
        "Signed update metadata is expired, mismatched or unsupported. No update was installed."
    case .untrustedDownload:
      return "The download does not match the signed size and checksum. It was rejected."
    case .unsupportedInstall:
      return
        "Shelf cannot replace this installation safely. Download the release manually, or use brew upgrade --cask jellydn/tap/shelf for Homebrew."
    case .download:
      return "The GitHub update check or download failed. Check your network and retry."
    case .process:
      return "App verification, installation or restart failed. No security controls were bypassed."
    }
  }
}

struct Manifest: Codable {
  let schema: Int
  let version: String
  let tag: String
  let channel: String
  let expires: Int
  let asset: String
  let size: Int
  let sha256: String
  func validate(tag expected: String, prereleases: Bool, now: Int) throws {
    let parsed = try UpdateVersion(version)
    guard schema == 1, tag == expected, tag == "v\(version)", expires > now,
      channel == (parsed.prerelease.isEmpty ? "stable" : "prerelease"),
      prereleases || parsed.prerelease.isEmpty,
      asset == "Shelf-\(version)-macos-arm64.zip", size > 0, size <= 536_870_912,
      sha256.count == 64, sha256.allSatisfy({ $0.isHexDigit && !$0.isUppercase })
    else { throw UpdateError.invalidMetadata }
  }
}

func verifiedManifest(
  _ data: Data, signature: Data, key: Data, tag: String, prereleases: Bool, now: Int
) throws -> Manifest {
  guard data.count <= 16_384, signature.count == 64,
    let publicKey = try? Curve25519.Signing.PublicKey(rawRepresentation: key),
    publicKey.isValidSignature(signature, for: data)
  else { throw UpdateError.invalidSignature }
  let manifest = try JSONDecoder().decode(Manifest.self, from: data)
  try manifest.validate(tag: tag, prereleases: prereleases, now: now)
  return manifest
}

func fileDigest(_ file: URL) throws -> (Int, String) {
  let input = try FileHandle(forReadingFrom: file)
  defer { try? input.close() }
  var hash = SHA256()
  var size = 0
  while let bytes = try input.read(upToCount: 1_048_576), !bytes.isEmpty {
    size += bytes.count
    guard size <= 536_870_912 else { throw UpdateError.untrustedDownload }
    hash.update(data: bytes)
  }
  return (size, hash.finalize().map { String(format: "%02x", $0) }.joined())
}

func verifyArchive(_ file: URL, manifest: Manifest) throws {
  let (size, digest) = try fileDigest(file)
  guard size == manifest.size, digest == manifest.sha256 else {
    throw UpdateError.untrustedDownload
  }
}

func isHomebrew(
  _ bundle: URL,
  caskrooms: [URL] = ["/opt/homebrew/Caskroom", "/usr/local/Caskroom"].map {
    URL(fileURLWithPath: $0)
  }
    + (ProcessInfo.processInfo.environment["HOMEBREW_PREFIX"].map {
      [URL(fileURLWithPath: $0).appendingPathComponent("Caskroom")]
    } ?? [])
) -> Bool {
  let target = bundle.resolvingSymlinksInPath()
  if target.pathComponents.contains("Caskroom") { return true }
  // Casks move the real app into Applications, then link Caskroom back to it.
  return caskrooms.contains { root in
    let versions = try? FileManager.default.contentsOfDirectory(
      at: root.appendingPathComponent("shelf"), includingPropertiesForKeys: nil)
    return versions?.contains {
      $0.appendingPathComponent("Shelf.app").resolvingSymlinksInPath() == target
    } ?? false
  }
}

// The verified app is staged on the destination filesystem before the running app closes.
func replaceBundle(staged: URL, target: URL, launch: (URL) throws -> Void) throws {
  let fm = FileManager.default
  let backup = target.deletingLastPathComponent().appendingPathComponent(
    ".Shelf-previous-\(UUID().uuidString).app")
  try fm.moveItem(at: target, to: backup)
  do {
    try fm.moveItem(at: staged, to: target)
    try launch(target)
  } catch {
    if fm.fileExists(atPath: target.path) { try fm.removeItem(at: target) }
    try fm.moveItem(at: backup, to: target)
    try? launch(target)
    throw UpdateError.process
  }
  // Keep the previous app for rollback. Settings and chats are never moved or removed.
}

func settings(_ path: URL) -> [String: String] {
  guard let text = try? String(contentsOf: path, encoding: .utf8) else { return [:] }
  return Dictionary(
    text.split(separator: "\n").compactMap { line in
      let pair = line.split(separator: "=", maxSplits: 1, omittingEmptySubsequences: false)
      return pair.count == 2 ? (String(pair[0]), String(pair[1])) : nil
    }, uniquingKeysWith: { _, last in last })
}

func run(_ executable: String, _ arguments: [String]) throws {
  let child = Process()
  child.executableURL = URL(fileURLWithPath: executable)
  child.arguments = arguments
  child.standardOutput = FileHandle.nullDevice
  child.standardError = FileHandle.nullDevice
  try child.run()
  child.waitUntilExit()
  guard child.terminationStatus == 0 else { throw UpdateError.process }
}

@MainActor
func notice(_ title: String, _ message: String, buttons: [String] = ["OK"]) -> Int {
  NSApplication.shared.setActivationPolicy(.accessory)
  NSApplication.shared.activate(ignoringOtherApps: true)
  let alert = NSAlert()
  alert.messageText = title
  alert.informativeText = message
  for button in buttons { alert.addButton(withTitle: button) }
  return alert.runModal().rawValue - NSApplication.ModalResponse.alertFirstButtonReturn.rawValue
}

@MainActor
struct Updater {
  let bundle: URL
  let prefsPath: URL
  let root: URL
  let manual: Bool
  let parent: pid_t
  let session: URLSession

  init(bundle: URL, prefsPath: URL, manual: Bool, parent: pid_t) {
    self.bundle = bundle.resolvingSymlinksInPath()
    self.prefsPath = prefsPath
    self.root = prefsPath.deletingLastPathComponent().appendingPathComponent(
      "Updates", isDirectory: true)
    self.manual = manual
    self.parent = parent
    let configuration = URLSessionConfiguration.ephemeral
    configuration.httpCookieStorage = nil
    configuration.urlCredentialStorage = nil
    configuration.timeoutIntervalForRequest = 30
    configuration.timeoutIntervalForResource = 300
    self.session = URLSession(configuration: configuration)
  }

  func data(_ url: URL, max: Int) async throws -> Data {
    var request = URLRequest(url: url)
    request.setValue(
      "Shelf/\(Bundle(path: bundle.path)?.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "unknown")",
      forHTTPHeaderField: "User-Agent")
    let (stream, response) = try await session.bytes(for: request)
    guard (response as? HTTPURLResponse)?.statusCode == 200 else { throw UpdateError.download }
    var bytes = Data()
    for try await byte in stream {
      guard bytes.count < max else { throw UpdateError.download }
      bytes.append(byte)
    }
    return bytes
  }

  func check() async throws {
    let fm = FileManager.default
    try fm.createDirectory(
      at: root, withIntermediateDirectories: true, attributes: [.posixPermissions: 0o700])
    // One helper per installation; automatic and manual checks must not race an installer.
    let descriptor = Darwin.open(
      root.appendingPathComponent("running.lock").path, O_CREAT | O_RDWR | O_NOFOLLOW, 0o600)
    guard descriptor >= 0 else { throw UpdateError.process }
    defer { Darwin.close(descriptor) }
    guard flock(descriptor, LOCK_EX | LOCK_NB) == 0 else {
      if manual {
        _ = notice(
          "Update check already running", "Wait for the current check or installer to finish.")
      }
      return
    }
    let prefs = settings(prefsPath)
    let stamp = root.appendingPathComponent("last-check")
    let now = Int(Date().timeIntervalSince1970)
    let last = Int((try? String(contentsOf: stamp, encoding: .utf8)) ?? "") ?? 0
    if !manual && (prefs["update_checks"] == "0" || now - last < 86_400) { return }
    // Bound failed attempts to daily too; manual checks remain available.
    try String(now).write(to: stamp, atomically: true, encoding: .utf8)
    let prereleases = prefs["update_prereleases"] == "1"
    let currentText =
      Bundle(path: bundle.path)?.object(forInfoDictionaryKey: "ShelfVersion") as? String ?? Bundle(
        path: bundle.path)?.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String
      ?? ""
    let current = try UpdateVersion(currentText)
    let keyText = try? String(
      contentsOf: bundle.appendingPathComponent("Contents/Resources/update-public-key.txt"),
      encoding: .utf8)
    guard
      let key = keyText.flatMap({
        Data(base64Encoded: $0.trimmingCharacters(in: .whitespacesAndNewlines))
      }), key.count == 32
    else { throw UpdateError.invalidSignature }
    let releaseData = try await data(
      URL(string: "https://api.github.com/repos/jellydn/shelf/releases?per_page=100")!,
      max: 2_097_152)
    guard let releases = try JSONSerialization.jsonObject(with: releaseData) as? [[String: Any]]
    else { throw UpdateError.download }
    let candidates = releases.compactMap { release -> (String, UpdateVersion)? in
      guard release["draft"] as? Bool == false,
        prereleases || release["prerelease"] as? Bool == false,
        let tag = release["tag_name"] as? String, tag.hasPrefix("v"),
        let version = try? UpdateVersion(String(tag.dropFirst())), version > current,
        prereleases || version.prerelease.isEmpty
      else { return nil }
      return (tag, version)
    }.sorted { $0.1 > $1.1 }
    guard let (tag, _) = candidates.first else {
      if manual {
        _ = notice("Shelf is up to date", "No newer release is available on the selected channel.")
      }
      return
    }
    // URLs are constructed, never taken from unsigned API metadata or the manifest.
    let base = URL(string: "https://github.com/jellydn/shelf/releases/download/\(tag)/")!
    let manifestData = try await data(base.appendingPathComponent("update.json"), max: 16_384)
    let signature = try await data(base.appendingPathComponent("update.json.sig"), max: 64)
    let manifest = try verifiedManifest(
      manifestData, signature: signature, key: key, tag: tag, prereleases: prereleases, now: now)
    if isHomebrew(bundle) {
      _ = notice(
        "Shelf \(manifest.version) is available",
        "This app is managed by Homebrew. Run:\nbrew upgrade --cask jellydn/tap/shelf\nThe stable cask does not install prereleases."
      )
      return
    }
    let archive = root.appendingPathComponent(manifest.asset)
    if (try? verifyArchive(archive, manifest: manifest)) == nil {
      if manual || prefs["update_downloads"] != "1" {
        guard
          notice(
            "Shelf \(manifest.version) is available",
            "Download a publisher-verified update? Installation and restart need separate confirmation. This build is ad-hoc signed, not notarized.",
            buttons: ["Download", "Later"]) == 0
        else { return }
      }
      let temporary = root.appendingPathComponent(UUID().uuidString)
      defer { try? fm.removeItem(at: temporary) }
      let (stream, response) = try await session.bytes(
        from: base.appendingPathComponent(manifest.asset))
      guard (response as? HTTPURLResponse)?.statusCode == 200 else { throw UpdateError.download }
      guard
        fm.createFile(atPath: temporary.path, contents: nil, attributes: [.posixPermissions: 0o600])
      else { throw UpdateError.download }
      let output = try FileHandle(forWritingTo: temporary)
      defer { try? output.close() }
      var chunk = Data()
      var received = 0
      for try await byte in stream {
        received += 1
        guard received <= manifest.size else { throw UpdateError.untrustedDownload }
        chunk.append(byte)
        if chunk.count == 65_536 {
          try output.write(contentsOf: chunk)
          chunk.removeAll(keepingCapacity: true)
        }
      }
      try output.write(contentsOf: chunk)
      try output.synchronize()
      try verifyArchive(temporary, manifest: manifest)
      if fm.fileExists(atPath: archive.path) { try fm.removeItem(at: archive) }
      try fm.moveItem(at: temporary, to: archive)
    }
    // An automatic download never grants installation, even when Shelf later quits.
    guard manual else { return }
    guard
      notice(
        "Install Shelf \(manifest.version)?",
        "Shelf will close and restart. Finish or cancel any agent review first. The downloaded archive is publisher-verified; macOS Gatekeeper may still require approval because the app is not notarized.",
        buttons: ["Install and Restart", "Not Now"]) == 0
    else { return }
    try await install(archive, manifest: manifest)
  }

  func install(_ archive: URL, manifest: Manifest) async throws {
    let fm = FileManager.default
    guard !isHomebrew(bundle), bundle.pathExtension == "app",
      !bundle.path.contains("/AppTranslocation/"),
      fm.isWritableFile(atPath: bundle.deletingLastPathComponent().path),
      let application = NSRunningApplication(processIdentifier: parent),
      application.bundleURL?.resolvingSymlinksInPath() == bundle
    else { throw UpdateError.unsupportedInstall }
    try verifyArchive(archive, manifest: manifest)
    let work = root.appendingPathComponent(UUID().uuidString, isDirectory: true)
    try fm.createDirectory(
      at: work, withIntermediateDirectories: false, attributes: [.posixPermissions: 0o700])
    defer { try? fm.removeItem(at: work) }
    try run("/usr/bin/ditto", ["-x", "-k", archive.path, work.path])
    let candidate = work.appendingPathComponent("Shelf.app")
    let info = Bundle(path: candidate.path)
    guard info?.bundleIdentifier == "io.github.jellydn.shelf",
      info?.object(forInfoDictionaryKey: "ShelfVersion") as? String == manifest.version,
      try Data(
        contentsOf: candidate.appendingPathComponent("Contents/Resources/update-public-key.txt"))
        == Data(
          contentsOf: bundle.appendingPathComponent("Contents/Resources/update-public-key.txt"))
    else { throw UpdateError.untrustedDownload }
    try run("/usr/bin/codesign", ["--verify", "--deep", "--strict", candidate.path])
    let staged = bundle.deletingLastPathComponent().appendingPathComponent(
      ".Shelf-update-\(UUID().uuidString).app")
    try fm.copyItem(at: candidate, to: staged)
    defer { try? fm.removeItem(at: staged) }
    try run("/usr/bin/codesign", ["--verify", "--deep", "--strict", staged.path])
    guard application.terminate() else { throw UpdateError.process }
    for _ in 0..<150 {
      if application.isTerminated { break }
      try await Task.sleep(for: .milliseconds(200))
    }
    guard application.isTerminated else { throw UpdateError.process }
    try replaceBundle(staged: staged, target: bundle) { app in
      try run("/usr/bin/open", [app.path])
    }
  }
}

@main
struct ShelfUpdater {
  @MainActor static func main() async {
    let args = CommandLine.arguments
    if args.count == 4 && args[1] == "--verify-release" {
      do {
        let directory = URL(fileURLWithPath: args[2])
        let keyText = try String(contentsOfFile: args[3], encoding: .utf8)
        guard let key = Data(base64Encoded: keyText.trimmingCharacters(in: .whitespacesAndNewlines))
        else { throw UpdateError.invalidSignature }
        let data = try Data(contentsOf: directory.appendingPathComponent("update.json"))
        let untrusted = try JSONDecoder().decode(Manifest.self, from: data)
        let manifest = try verifiedManifest(
          data, signature: Data(contentsOf: directory.appendingPathComponent("update.json.sig")),
          key: key, tag: untrusted.tag, prereleases: true, now: Int(Date().timeIntervalSince1970))
        try verifyArchive(directory.appendingPathComponent(manifest.asset), manifest: manifest)
        let publicKey = try Curve25519.Signing.PublicKey(rawRepresentation: key)
        for name in [manifest.asset, "Shelf-\(manifest.version)-macos-arm64.dmg"] {
          guard
            try publicKey.isValidSignature(
              Data(contentsOf: directory.appendingPathComponent(name + ".sig")),
              for: Data(contentsOf: directory.appendingPathComponent(name)))
          else { throw UpdateError.invalidSignature }
        }
        print("Publisher metadata and both assets verified")
      } catch {
        fputs("Release publisher verification failed\n", stderr)
        exit(1)
      }
      return
    }
    if args.count == 2 && args[1] == "--self-test" {
      do {
        try selfTest()
        print("Updater verification tests passed")
      } catch {
        fputs("Updater test failed: \(error)\n", stderr)
        exit(1)
      }
      return
    }
    guard args.count == 5, let pid = pid_t(args[4]), pid > 0 else { exit(2) }
    let manual = args[1] == "--manual"
    guard manual || args[1] == "--automatic" else { exit(2) }
    let updater = Updater(
      bundle: URL(fileURLWithPath: args[2]), prefsPath: URL(fileURLWithPath: args[3]),
      manual: manual, parent: pid)
    do { try await updater.check() } catch {
      // Do not expose paths, provider context, credentials, URLs or raw transport errors.
      if manual {
        _ = notice(
          "Shelf update unavailable",
          (error as? UpdateError)?.errorDescription
            ?? "Update verification failed. Check your network and retry. No update was installed.")
      }
    }
  }
}

func selfTest() throws {
  func require(_ value: Bool) { precondition(value) }
  func reject(_ body: () throws -> Void) {
    do {
      try body()
      preconditionFailure("Expected rejection")
    } catch {}
  }
  require(try UpdateVersion("0.2.0-beta.2") < UpdateVersion("0.2.0-beta.10"))
  require(try UpdateVersion("0.2.0-beta.10") < UpdateVersion("0.2.0"))
  require(try UpdateVersion("0.2.0") < UpdateVersion("0.10.0"))
  reject { _ = try UpdateVersion("../0.2.0") }
  reject { _ = try UpdateVersion("0.2.0-") }
  let key = Curve25519.Signing.PrivateKey()
  let bytes = Data("harmless update fixture".utf8)
  let digest = SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined()
  let manifest = Manifest(
    schema: 1, version: "0.2.0-beta.1", tag: "v0.2.0-beta.1", channel: "prerelease", expires: 200,
    asset: "Shelf-0.2.0-beta.1-macos-arm64.zip", size: bytes.count, sha256: digest)
  let data = try JSONEncoder().encode(manifest)
  let signature = try key.signature(for: data)
  let verified = try verifiedManifest(
    data, signature: signature, key: key.publicKey.rawRepresentation, tag: manifest.tag,
    prereleases: true, now: 100)
  require(verified.version == manifest.version)
  reject {
    _ = try verifiedManifest(
      data + Data(" ".utf8), signature: signature, key: key.publicKey.rawRepresentation,
      tag: manifest.tag, prereleases: true, now: 100)
  }
  reject {
    _ = try verifiedManifest(
      data, signature: Data(repeating: 0, count: 64), key: key.publicKey.rawRepresentation,
      tag: manifest.tag, prereleases: true, now: 100)
  }
  reject {
    _ = try verifiedManifest(
      data, signature: signature, key: Curve25519.Signing.PrivateKey().publicKey.rawRepresentation,
      tag: manifest.tag, prereleases: true, now: 100)
  }
  reject {
    _ = try verifiedManifest(
      data, signature: signature, key: key.publicKey.rawRepresentation, tag: manifest.tag,
      prereleases: false, now: 100)
  }
  reject {
    _ = try verifiedManifest(
      data, signature: signature, key: key.publicKey.rawRepresentation, tag: manifest.tag,
      prereleases: true, now: 201)
  }
  reject {
    _ = try verifiedManifest(
      data, signature: signature, key: key.publicKey.rawRepresentation, tag: "v0.3.0",
      prereleases: true, now: 100)
  }
  let temporary = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
  defer { try? FileManager.default.removeItem(at: temporary) }
  try bytes.write(to: temporary)
  try verifyArchive(temporary, manifest: manifest)
  try (bytes + Data("tampered".utf8)).write(to: temporary)
  reject { try verifyArchive(temporary, manifest: manifest) }
  require(isHomebrew(URL(fileURLWithPath: "/opt/homebrew/Caskroom/shelf/0.1.0/Shelf.app")))
  require(!isHomebrew(URL(fileURLWithPath: "/Applications/Shelf.app"), caskrooms: []))
  let transaction = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
  try FileManager.default.createDirectory(at: transaction, withIntermediateDirectories: false)
  defer { try? FileManager.default.removeItem(at: transaction) }
  let apps = transaction.appendingPathComponent("Applications/Shelf.app")
  let caskroom = transaction.appendingPathComponent("Caskroom")
  let receipt = caskroom.appendingPathComponent("shelf/0.1.0/Shelf.app")
  try FileManager.default.createDirectory(at: apps, withIntermediateDirectories: true)
  try FileManager.default.createDirectory(
    at: receipt.deletingLastPathComponent(), withIntermediateDirectories: true)
  require(!isHomebrew(apps, caskrooms: [caskroom]))
  try FileManager.default.createSymbolicLink(at: receipt, withDestinationURL: apps)
  require(isHomebrew(apps, caskrooms: [caskroom]))
  require(!isHomebrew(transaction.appendingPathComponent("Other.app"), caskrooms: [caskroom]))
  let target = transaction.appendingPathComponent("Shelf.app")
  let staged = transaction.appendingPathComponent("staged.app")
  try Data("old".utf8).write(to: target)
  try Data("new".utf8).write(to: staged)
  reject { try replaceBundle(staged: staged, target: target) { _ in throw UpdateError.process } }
  require(try Data(contentsOf: target) == Data("old".utf8))
  try Data("new".utf8).write(to: staged)
  try replaceBundle(staged: staged, target: target) { _ in }
  require(try Data(contentsOf: target) == Data("new".utf8))
}
