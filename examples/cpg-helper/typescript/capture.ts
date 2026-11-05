import { spawn } from 'node:child_process';

/** Each document is an async iterable of UTF-8 JSON byte chunks, beginning with `{`.
 * Callers generate records incrementally; this adapter never JSON.stringify's a graph.
 * No Zstandard package or Rust toolchain is needed: use the installed cargo-rstyle.
 */
export async function writeCapture(
  executable: string,
  output: string,
  documents: AsyncIterable<AsyncIterable<Uint8Array>>,
): Promise<void> {
  const child = spawn(executable, ['rstyle', 'cpg', 'ingest', '--out', output], {
    stdio: ['pipe', 'inherit', 'inherit'],
  });
  const completed = new Promise<void>((resolve, reject) => {
    child.once('error', reject);
    child.once('exit', (code, signal) => code === 0 ? resolve()
      : reject(new Error(`CPG helper failed: ${signal ?? code}`)));
  });
  // Writes report errors through their callbacks. Keep the process alive long
  // enough to close stdin and let the helper remove its staged capture.
  completed.catch(() => {});
  child.stdin.on('error', () => {});
  const write = (bytes: Uint8Array) => new Promise<void>((resolve, reject) => {
    child.stdin.write(bytes, error => error ? reject(error) : resolve());
  });
  try {
    for await (const document of documents) {
      await write(Uint8Array.of(0x1e));
      for await (const chunk of document) await write(chunk);
      await write(Uint8Array.of(0x0a));
    }
    await write(Buffer.from('\x1enull\n'));
    child.stdin.end();
    await completed;
  } catch (error) {
    child.stdin.destroy();
    await completed.catch(() => {});
    throw error;
  }
}
