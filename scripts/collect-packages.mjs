import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import os from 'node:os';
import { basename, join, resolve } from 'node:path';

const artifacts = resolve('artifacts');
mkdirSync(artifacts, { recursive: true });
const extension = { linux: '.deb', win32: '.exe', darwin: '.dmg' }[process.platform];
assert(extension);
if (process.platform !== 'darwin') {
  const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--no-deps', '--format-version', '1'], { encoding: 'utf8' }));
  const bundle = join(metadata.target_directory, 'release/bundle', process.platform === 'win32' ? 'nsis' : 'deb');
  for (const entry of readdirSync(bundle, { recursive: true, withFileTypes: true })) {
    if (entry.isFile() && entry.name.endsWith(extension)) {
      const source = join(entry.parentPath, entry.name);
      cpSync(source, join(artifacts, basename(source)), { errorOnExist: true, force: false });
    }
  }
}
const packages = readdirSync(artifacts).filter((name) => name.endsWith(extension));
assert(packages.length > 0, 'No package was produced.');
const sums = packages.map((name) => `${createHash('sha256').update(readFileSync(join(artifacts, name))).digest('hex')}  ${name}`);
writeFileSync(join(artifacts, 'SHA256SUMS.txt'), sums.join('\n') + '\n');
const runtime = JSON.parse(readFileSync('scripts/webview2-runtime.json', 'utf8'));
writeFileSync(join(artifacts, 'build-info.json'), JSON.stringify({
  commit: process.env.GITHUB_SHA ?? execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
  platform: process.platform,
  architecture: process.arch,
  operatingSystem: os.version(),
  kernel: os.release(),
  webkitGtk: process.platform === 'linux'
    ? execFileSync('dpkg-query', ['-W', '-f=${Version}', 'libwebkit2gtk-4.1-0'], { encoding: 'utf8' })
    : null,
  webview2: process.platform === 'win32' ? { version: runtime.version, sha256: runtime.sha256 } : null,
  distributionSigning: 'Developer distribution signing and notarization have not been performed.',
}, null, 2) + '\n');
console.log(sums.join('\n'));
