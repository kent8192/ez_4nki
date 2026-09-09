import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, rmSync, symlinkSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

if (process.platform !== 'darwin') throw new Error('Build this package on macOS.');
const run = (cmd, args) => execFileSync(cmd, args, { stdio: 'inherit' });
run('npm', ['run', 'tauri', '--', 'build', '--bundles', 'app']);
const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--no-deps', '--format-version', '1'], { encoding: 'utf8' }));
const source = join(metadata.target_directory, 'release/bundle/macos/Kotoba.app');
const artifacts = resolve('artifacts');
mkdirSync(artifacts, { recursive: true });
run('ditto', [source, join(artifacts, 'Kotoba.app')]);
const stage = mkdtempSync(join(tmpdir(), 'kotoba-dmg-'));
try {
  run('ditto', [source, join(stage, 'Kotoba.app')]);
  symlinkSync('/Applications', join(stage, 'Applications'));
  run('hdiutil', ['create', '-ov', '-volname', 'Kotoba', '-srcfolder', stage, '-format', 'UDZO', join(artifacts, `Kotoba_0.1.0_${process.arch}.dmg`)]);
} finally {
  rmSync(stage, { recursive: true, force: true });
}
