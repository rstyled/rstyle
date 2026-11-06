import Foundation

enum CaptureError: Error {
    case invalidDocumentState
    case helperFailed(Int32)
}

/// Feed UTF-8 JSON chunks incrementally. Zstandard and manifests belong to the helper.
final class CaptureWriter {
    private let input: FileHandle
    private var open = false

    fileprivate init(input: FileHandle) { self.input = input }

    func beginDocument() throws {
        guard !open else { throw CaptureError.invalidDocumentState }
        try input.write(contentsOf: Data([0x1e]))
        open = true
    }

    func write(_ chunk: Data) throws {
        guard open else { throw CaptureError.invalidDocumentState }
        try input.write(contentsOf: chunk)
    }

    func endDocument() throws {
        guard open else { throw CaptureError.invalidDocumentState }
        try input.write(contentsOf: Data([0x0a]))
        open = false
    }

    fileprivate func finish() throws {
        guard !open else { throw CaptureError.invalidDocumentState }
        try input.write(contentsOf: Data([0x1e]) + Data("null\n".utf8))
        try input.close()
    }
}

/// Launch one installed helper per capture; no Rust toolchain or Zstd dependency.
func withCPGCapture(
    executable: URL,
    output: URL,
    emit: (CaptureWriter) throws -> Void
) throws {
    let process = Process()
    let pipe = Pipe()
    process.executableURL = executable
    process.arguments = ["rstyle", "cpg", "ingest", "--out", output.path]
    process.standardInput = pipe
    defer {
        try? pipe.fileHandleForWriting.close()
        try? pipe.fileHandleForReading.close()
    }
    try process.run()
    try pipe.fileHandleForReading.close()
    do {
        let writer = CaptureWriter(input: pipe.fileHandleForWriting)
        try emit(writer)
        try writer.finish()
        process.waitUntilExit()
        guard process.terminationStatus == 0 else {
            throw CaptureError.helperFailed(process.terminationStatus)
        }
    } catch {
        // EOF lets the helper remove its staged output. Killing it would bypass cleanup.
        try? pipe.fileHandleForWriting.close()
        process.waitUntilExit()
        throw error
    }
}
