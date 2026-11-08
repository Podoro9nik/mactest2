// Stage only reviewed native sources for the dedicated build repository.
import { cp, mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { resolve, join, relative, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const target = resolve(process.argv[2] || join(root,'build/mactest2-ci'));
if (target === root || !target.startsWith(join(root,'build') + '\\') && !target.startsWith(join(root,'build') + '/'))
  throw new Error('The staging checkout must be below this workspace build directory.');
const excluded = new Set(['node_modules','target','build','.gradle','.kotlin','.git','.idea','verification','encrypted-artifacts']);
const files = [];
async function visit(path) {
  for (const item of await readdir(path,{withFileTypes:true})) {
    const source = join(path,item.name), rel = relative(root,source).replaceAll('\\','/');
    if (excluded.has(item.name) || item.name === 'local.properties' || /\.(jks|keystore|log|enc|exe|apk|dmg|zip)$/.test(item.name) || rel.includes('/resources/bin/')) continue;
    if (item.isDirectory()) await visit(source);
    else files.push({source,rel});
  }
}
await visit(join(root,'native'));
for(const rel of ['.github/workflows/native-build.yml','.gitignore','CODEX-LICENSE.txt','CODEX-NOTICE.txt']) files.push({source:join(root,rel),rel});
for(const {source,rel} of files) {
  const bytes = await readFile(source);
  if (/(?:ek_mcp_|ek_live_)[A-Za-z0-9_-]{16,}/.test(bytes.toString('utf8'))) throw new Error(`Credential found in source: ${rel}`);
  await mkdir(dirname(join(target,rel)),{recursive:true});
  await cp(source,join(target,rel));
}
await writeFile(join(target,'README.md'),await readFile(join(root,'native/README.md')));
console.log(`Staged ${files.length} native source files; credential scan passed.`);
