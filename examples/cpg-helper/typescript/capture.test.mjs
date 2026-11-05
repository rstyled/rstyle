import assert from 'node:assert/strict';
import { createReadStream } from 'node:fs';
import { mkdtemp, readFile, readdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import { writeCapture } from './capture.ts';

const binary = process.env.RSTYLE_CPG_BIN;
if (!binary) throw new Error('Set RSTYLE_CPG_BIN to the built cargo-rstyle executable');
const fixture = new URL('../../../crates/rstyle-core/src/cpg/testdata/example.cpg.json', import.meta.url);

test('streams a real CPG document into a capture that Rust can analyze', async () => {
  const root = await mkdtemp(join(tmpdir(), 'cpg-node-'));
  try {
    const output = join(root, 'capture');
    async function* documents() { yield createReadStream(fixture, { highWaterMark: 37 }); }
    await writeCapture(binary, output, documents());
    const manifest = JSON.parse(await readFile(join(output, 'documents.manifest.json'), 'utf8'));
    assert.equal(manifest.documents.length, 1);
    const check = spawnSync(binary, ['rstyle', 'cpg', 'check', '--format', 'json', join(output, manifest.documents[0].file)], { encoding: 'utf8' });
    assert.equal(check.status, 0, check.stderr);
    assert.equal(JSON.parse(check.stdout).summary.functions, 3);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('a frontend exception closes stdin and leaves no published or staged capture', async () => {
  const root = await mkdtemp(join(tmpdir(), 'cpg-node-'));
  try {
    async function* broken() { yield Buffer.from('{'); throw new Error('frontend failed'); }
    async function* documents() { yield broken(); }
    await assert.rejects(writeCapture(binary, join(root, 'capture'), documents()), /frontend failed/);
    assert.deepEqual(await readdir(root), []);
  } finally { await rm(root, { recursive: true, force: true }); }
});

test('helper startup failure rejects rather than hanging', async () => {
  async function* documents() { yield (async function* () { yield Buffer.from('{}'); })(); }
  await assert.rejects(writeCapture('/missing/cpg-helper', '/unused', documents()));
});
