import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile, copyFile, chmod, readdir } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');
const destination = join(root, 'native/desktop/resources/bin');
const version = '0.147.0';
const target = process.platform === 'darwin'
  ? `${process.arch === 'arm64' ? 'aarch64' : 'x86_64'}-apple-darwin`
  : 'x86_64-pc-windows-msvc';
const extension = process.platform === 'win32' ? '.exe' : '';
const checksums = {
  'codex-aarch64-apple-darwin.tar.gz': '75984b81f92a71b0c0f4b3b5cad80e5c57177e4d8c8b4b1e13db703b20dc4358',
  'codex-x86_64-apple-darwin.tar.gz': '36e782f71d8164cc37c2b89c64948f2180e9a2f8456b27e660da75bc6b5574e2',
  'codex-code-mode-host-aarch64-apple-darwin.tar.gz': '56cdbf6187bf914108d3b7feeea5a34ffba15e5c162bedce69e062ee92ddfb5e',
  'codex-code-mode-host-x86_64-apple-darwin.tar.gz': '7131a0508de4dea60f79c816188b0b06b17f6ed417d9b3a1865b0a4927fbc48a',
  'codex-x86_64-pc-windows-msvc.exe.tar.gz': '7c47d4dc9202569d85ef2cbe9203dd2e75c326f5085e3854487a740339a8bdcd',
  'codex-code-mode-host-x86_64-pc-windows-msvc.exe.tar.gz': '9d1befd66627cd719e4fa69a58a7ac21aafc3c016d13116ece4e4b79904d95b8'
};
if (!['win32','darwin'].includes(process.platform)) throw new Error('This distribution supports Windows and macOS.');
await mkdir(destination, {recursive:true});
for (const name of ['codex','codex-code-mode-host']) {
  const output = join(destination, name + extension);
  const legacy = join(root, 'dist/Chiikawizator/_internal/bin', name + extension);
  if (process.platform === 'win32' && existsSync(legacy)) {
    await copyFile(legacy, output);
  } else {
    const asset = `${name}-${target}${extension}.tar.gz`;
    const cache = join(root,'build/codex-download',target,name);
    await mkdir(cache,{recursive:true});
    const archive = join(cache, asset);
    if (!existsSync(archive)) {
      const response = await fetch(`https://github.com/openai/codex/releases/download/rust-v${version}/${asset}`);
      if (!response.ok) throw new Error(`Cannot download Codex: ${response.status}`);
      await writeFile(archive, new Uint8Array(await response.arrayBuffer()));
    }
    const digest = createHash('sha256').update(await readFile(archive)).digest('hex');
    if (digest !== checksums[asset]) throw new Error(`Codex archive checksum mismatch: ${asset}`);
    execFileSync('tar',['-xzf',archive,'-C',cache]);
    const files = await readdir(cache);
    const binary = files.find(file => file === `${name}-${target}${extension}` || file === name + extension);
    if (!binary) throw new Error(`Codex binary absent in ${asset}`);
    await copyFile(join(cache,binary), output);
  }
  if (process.platform !== 'win32') await chmod(output,0o755);
}
for (const file of ['CODEX-LICENSE.txt','CODEX-NOTICE.txt']) await copyFile(join(root,file),join(destination,file));
const actual = execFileSync(join(destination,'codex'+extension),['--version'],{encoding:'utf8'}).trim();
if (!actual.endsWith(version)) throw new Error(`Unexpected Codex version: ${actual}`);
console.log(`Bundled ${actual} for ${target}.`);
