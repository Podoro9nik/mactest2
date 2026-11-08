// Public CI stores only encrypted personal installers. The build key never enters logs.
import { createCipheriv, createDecipheriv, randomBytes, scryptSync } from 'node:crypto';
import { createReadStream, createWriteStream } from 'node:fs';
import { open, stat, unlink } from 'node:fs/promises';
import { pipeline } from 'node:stream/promises';
const [operation, input, output] = process.argv.slice(2);
const secret = process.env.CHIIKAWA_EMBEDDED_MCP_KEY;
if (!secret || !input || !output || !['encrypt','decrypt'].includes(operation))
  throw new Error('Use encrypt|decrypt INPUT OUTPUT with CHIIKAWA_EMBEDDED_MCP_KEY set.');
const magic = Buffer.from('CHIIKAWA1');
if (operation === 'encrypt') {
  const salt = randomBytes(16), iv = randomBytes(12);
  const cipher = createCipheriv('aes-256-gcm', scryptSync(secret,salt,32),iv);
  const header = Buffer.concat([magic,salt,iv]);
  const handle = await open(output,'w');
  await handle.write(header); await handle.close();
  await pipeline(createReadStream(input),cipher,createWriteStream(output,{flags:'a'}));
  const append = await open(output,'a'); await append.write(cipher.getAuthTag()); await append.close();
} else {
  const handle = await open(input,'r');
  const header = Buffer.alloc(37); await handle.read(header,0,37,0);
  if (!header.subarray(0,9).equals(magic)) throw new Error('Invalid encrypted installer.');
  const length = (await stat(input)).size;
  const tag = Buffer.alloc(16); await handle.read(tag,0,16,length-16); await handle.close();
  const decipher = createDecipheriv('aes-256-gcm',scryptSync(secret,header.subarray(9,25),32),header.subarray(25,37));
  decipher.setAuthTag(tag);
  try {
    await pipeline(createReadStream(input,{start:37,end:length-17}),decipher,createWriteStream(output));
  } catch {
    await unlink(output).catch(()=>{});
    throw new Error('Installer could not be authenticated; check the build key.');
  }
}
console.log(operation === 'encrypt' ? 'Personal installer encrypted.' : 'Personal installer restored.');
