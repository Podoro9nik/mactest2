// Failure diagnostics are uploaded only after this completes successfully.
import { lstat, mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { extname, join, resolve } from 'node:path';
const [directory] = process.argv.slice(2);
if (!directory) throw new Error('Use sanitize-verification.mjs VERIFICATION_DIRECTORY');
const root = resolve(directory);
await mkdir(root, { recursive: true });
const secret = process.env.CHIIKAWA_EMBEDDED_MCP_KEY;
const tokens = /(?:ek_mcp_|ek_live_)[A-Za-z0-9_-]{16,}|gh[pousr]_[A-Za-z0-9_]{20,}/g;
const textTypes = new Set(['.json', '.jsonl', '.log', '.txt']);
const reviewed = [];
function redact(text) {
  const explicit = secret ? text.split(secret).join('[redacted]') : text;
  return explicit.replace(tokens, '[redacted]');
}
async function scan(path) {
  const info = await lstat(path);
  if (info.isSymbolicLink()) throw new Error('Symlinks are not permitted in verification artifacts.');
  if (info.isDirectory()) {
    for (const name of await readdir(path)) await scan(join(path, name));
    return;
  }
  if (!info.isFile()) throw new Error('Unexpected verification artifact type.');
  const extension = extname(path).toLowerCase();
  if (!textTypes.has(extension) && extension !== '.png') throw new Error('Unexpected verification artifact extension.');
  const bytes = await readFile(path);
  const text = bytes.toString('utf8');
  if (textTypes.has(extension)) reviewed.push({ path, content: redact(text) });
  else {
    if (!bytes.subarray(0, 8).equals(Buffer.from([137,80,78,71,13,10,26,10]))) throw new Error('Invalid verification image.');
    if (redact(text) !== text) throw new Error('Credential content found in a verification image.');
  }
}
await scan(root);
for (const entry of reviewed) await writeFile(entry.path, entry.content);
await writeFile(join(root, 'report-safety.json'), JSON.stringify({ sanitized: true, reviewedTextFiles: reviewed.length }) + '\n');
console.log(`Verification reports sanitized: ${reviewed.length} text files.`);
