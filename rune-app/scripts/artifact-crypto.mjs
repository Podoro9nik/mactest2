// Only authenticated, encrypted personal installers are uploaded from public CI.
import { createCipheriv, createDecipheriv, randomBytes, scryptSync } from 'node:crypto';
import { createReadStream, createWriteStream } from 'node:fs';
import { open, stat, unlink } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pipeline } from 'node:stream/promises';

const [operation, input, output] = process.argv.slice(2);
const secret = process.env.CHIIKAWA_EMBEDDED_MCP_KEY;
if (!secret || !input || !output || !['encrypt', 'decrypt'].includes(operation))
  throw new Error('Use encrypt|decrypt INPUT OUTPUT with CHIIKAWA_EMBEDDED_MCP_KEY set.');
if (resolve(input) === resolve(output)) throw new Error('Input and output must be different files.');
const magic = Buffer.from('CHIIKAWA1');
// Reserve the output exclusively: an accidental path must never overwrite a source or installer.
const reserved = await open(output, 'wx', 0o600);
await reserved.close();
try {
  if (operation === 'encrypt') {
    const salt = randomBytes(16), iv = randomBytes(12);
    const cipher = createCipheriv('aes-256-gcm', scryptSync(secret, salt, 32), iv);
    const handle = await open(output, 'w', 0o600);
    await handle.write(Buffer.concat([magic, salt, iv]));
    await handle.close();
    await pipeline(createReadStream(input), cipher, createWriteStream(output, { flags: 'a' }));
    const append = await open(output, 'a');
    await append.write(cipher.getAuthTag());
    await append.close();
  } else {
    const length = (await stat(input)).size;
    if (length < 53) throw new Error('Invalid encrypted installer.');
    const handle = await open(input, 'r');
    const header = Buffer.alloc(37), tag = Buffer.alloc(16);
    try {
      await handle.read(header, 0, 37, 0);
      await handle.read(tag, 0, 16, length - 16);
    } finally { await handle.close(); }
    if (!header.subarray(0, 9).equals(magic)) throw new Error('Invalid encrypted installer.');
    const decipher = createDecipheriv('aes-256-gcm', scryptSync(secret, header.subarray(9, 25), 32), header.subarray(25, 37));
    decipher.setAuthTag(tag);
    await pipeline(createReadStream(input, { start: 37, end: length - 17 }), decipher, createWriteStream(output, { mode: 0o600 }));
  }
} catch {
  await unlink(output).catch(() => {});
  throw new Error('Installer encryption/authentication failed; check the input and build key.');
}
console.log(operation === 'encrypt' ? 'Personal installer encrypted.' : 'Personal installer restored.');
