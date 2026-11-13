// Publish only the Rune application sources, never the local checkout or personal build outputs.
import { lstat, mkdir, readdir, readFile, realpath, writeFile } from 'node:fs/promises';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const buildRoot = join(repository, 'build');
const target = resolve(process.argv[2] || join(buildRoot, 'rune-ci-stage'));
const fromBuild = relative(buildRoot, target);
if (!fromBuild || fromBuild.startsWith('..') || isAbsolute(fromBuild))
  throw new Error('The staging checkout must be inside this workspace build directory.');
await mkdir(buildRoot, { recursive: true });
await mkdir(target, { recursive: true });
const realBuild = await realpath(buildRoot), realTarget = await realpath(target);
const realRelative = relative(realBuild, realTarget);
if (!realRelative || realRelative.startsWith('..') || isAbsolute(realRelative))
  throw new Error('The staging checkout resolves outside the workspace build directory.');
const excluded = new Set(['vendor', 'target', 'build', 'dist', 'node_modules', '.git', 'verification', 'encrypted-artifacts', '.env', '.DS_Store']);
const files = [];
async function visit(directory) {
  for (const item of await readdir(directory, { withFileTypes: true })) {
    const name = item.name.toLowerCase();
    if (excluded.has(name) || name.startsWith('.env') || name.includes('.secret.') || /\.(?:exe|dll|dmg|apk|zip|enc|log|pdb|key|p12|pem|pfx|jks|keystore)$/i.test(name)) continue;
    const source = join(directory, item.name);
    // In particular, never traverse the local Rune directory junction.
    if (item.isSymbolicLink() || (await lstat(source)).isSymbolicLink()) continue;
    if (item.isDirectory()) await visit(source);
    else if (item.isFile()) files.push({ source, rel: relative(repository, source).replaceAll('\\', '/') });
  }
}
await visit(join(repository, 'rune-app'));
files.push({ source: join(repository, '.github/workflows/rune-build.yml'), rel: '.github/workflows/rune-build.yml' });
files.push({ rel: 'README.md', bytes: Buffer.from(`# Chiikawizator\n\nNative Windows and macOS application. Русский · English · 日本語.\n\n- [Использование / User guide / 使い方](rune-app/README.md)\n- [Build and personal artifact instructions](rune-app/BUILDING.md)\n\nThe build workflow publishes encrypted personal installers and offline verification reports.\n`) });
// Validate the complete selection before copying any file into the publication checkout.
for (const file of files) {
  file.bytes ??= await readFile(file.source);
  const text = file.bytes.toString('utf8');
  if (/(?:ek_mcp_|ek_live_)[A-Za-z0-9_-]{16,}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/.test(text))
    throw new Error(`Credential found in source: ${file.rel}`);
  const secret = process.env.CHIIKAWA_EMBEDDED_MCP_KEY;
  if (secret && text.includes(secret)) throw new Error(`Build credential found in source: ${file.rel}`);
}
async function inspect(path) {
  try { return await lstat(path); }
  catch (error) { if (error.code === 'ENOENT') return null; throw error; }
}
async function ensureDirectory(path) {
  let current = target;
  for (const segment of relative(target, path).split(/[\\/]/).filter(Boolean)) {
    if (segment === '..') throw new Error('Destination escaped the staging directory.');
    current = join(current, segment);
    const info = await inspect(current);
    if (info && (info.isSymbolicLink() || !info.isDirectory())) throw new Error('Unsafe staging directory entry.');
    if (!info) await mkdir(current);
  }
}
for (const { bytes, rel } of files) {
  const destination = join(target, rel);
  await ensureDirectory(dirname(destination));
  const info = await inspect(destination);
  if (info && (info.isSymbolicLink() || !info.isFile())) throw new Error(`Unsafe staging file: ${rel}`);
  // Write the bytes that passed the scan, even if a local editor changes its source meanwhile.
  await writeFile(destination, bytes);
}
const manifestInfo = await inspect(join(target, 'rune-source-manifest.json'));
if (manifestInfo && (manifestInfo.isSymbolicLink() || !manifestInfo.isFile())) throw new Error('Unsafe staging manifest path.');
await writeFile(join(target, 'rune-source-manifest.json'), JSON.stringify({
  version: '4.1.0', runeRevision: 'ea6ddf8f2c3cd476a3797caad7d77cbe04641d9c',
  files: files.map(file => file.rel).sort(),
}, null, 2) + '\n');
console.log(`Staged ${files.length} Rune source files; credential scan passed. Nothing was pushed.`);
