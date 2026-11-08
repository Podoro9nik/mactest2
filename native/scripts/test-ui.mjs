import { chromium, expect } from '@playwright/test';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve, join, extname, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const ui = join(root,'ui');
const mime = {'.html':'text/html','.css':'text/css','.js':'text/javascript','.png':'image/png','.svg':'image/svg+xml','.ttf':'font/ttf'};
const server = createServer(async(req,res)=>{
  const path = resolve(ui, '.' + new URL(req.url,'http://127.0.0.1').pathname.replace(/\/$/,'/index.html'));
  if(!path.startsWith(ui)) {res.writeHead(403).end();return;}
  try {res.setHeader('Content-Type',mime[extname(path)]||'application/octet-stream');res.end(await readFile(path));}
  catch {res.writeHead(404).end();}
});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const address=`http://127.0.0.1:${server.address().port}`;
const browser=await chromium.launch({...(process.env.CHIIKAWA_TEST_CHROME ? {executablePath:process.env.CHIIKAWA_TEST_CHROME} : {}),headless:true});
const page=await browser.newPage({viewport:{width:1280,height:850}});
const errors=[];page.on('pageerror',error=>errors.push(error.message));
const image='data:image/png;base64,'+(await readFile(join(root,'shared/assets/trio-fullbody.png'))).toString('base64');
await page.addInitScript(({image})=>{
  const calls=[]; const jobs=[];
  window.testCalls=calls;
  window.__TAURI__={core:{invoke:async(command,args={})=>{
    calls.push({command,args});
    if(command==='app_info') return {version:'3.0.0',outputDir:'Test outputs',hasEmbeddedKey:true,codexAvailable:true,smokeTest:false};
    if(command==='account') return {balance:100,plan:'Pro'};
    if(command==='codex_account') return {authenticated:true,plan:'Pro',imageGeneration:true};
    if(command==='list_jobs') return jobs;
    if(command==='generate') {
      const job={id:'local-'+(jobs.length+1),provider:args.request.provider,status:'queued',createdAt:new Date().toISOString(),description:args.request.description};
      jobs.unshift(job);return {...job};
    }
    if(command==='get_job') {
      const job=jobs.find(j=>j.id===args.jobId);
      if(!job) throw 'invalid_job';
      if(job.status==='queued'||job.status==='running') Object.assign(job,{status:job.provider==='elyum'?'preview':'completed',imageUrl:image,cost:6});
      return {...job};
    }
    if(command==='keep_original') {
      const job=jobs.find(j=>j.id===args.jobId); Object.assign(job,{status:'completed',imageUrl:image}); return {...job};
    }
    if(command==='restore_job') {
      const job={id:'restored',provider:'elyum',status:'preview',imageUrl:image,cost:9,createdAt:new Date().toISOString(),remoteId:args.jobId};
      jobs.unshift(job);return job;
    }
    if(command==='export_image') return '/test/result.png';
    if(command==='codex_login') return {message:'Sign in started'};
    throw new Error('Unexpected command '+command);
  }}};
},{image});
const report={ok:false,checks:[]};
try {
  await page.goto(address); await page.locator('#language').selectOption('ru');
  await page.locator('.ideas summary').click();
  await page.locator('#idea-buttons button').first().click();
  expect(await page.locator('#description').inputValue()).not.toBe('');
  report.checks.push('Inspiration fills description');
  for(const locale of ['en','ja','ru']) {
    await page.locator('#language').selectOption(locale);
    await expect(page.locator('html')).toHaveAttribute('lang',locale);
    await page.locator('#provider').selectOption('codex');
    await expect(page.locator('#codex-options')).toBeVisible();
    await expect(page.locator('#elyum-options')).toBeHidden();
    await page.locator('#provider').selectOption('elyum');
    await expect(page.locator('#elyum-options')).toBeVisible();
    await expect(page.locator('#codex-options')).toBeHidden();
  }
  report.checks.push('Three locales and provider panels remain separate');
  await page.locator('#mode-photo').click();
  await page.locator('#photos').setInputFiles([join(root,'shared/assets/chiikawa.png'),join(root,'shared/assets/hachiware.png')]);
  await expect(page.locator('#photo-thumbnails img')).toHaveCount(2);
  await page.locator('input[name=photoStyle][value=creature]').check();
  await page.locator('#description').fill('Two friends having tea');
  await page.locator('#generate').click();
  await expect(page.locator('#result-image')).toBeVisible({timeout:20000});
  await expect(page.locator('#keep')).toBeVisible();
  expect(await page.evaluate(()=>window.testCalls.filter(c=>c.command==='keep_original').length)).toBe(0);
  const request=await page.evaluate(()=>window.testCalls.find(c=>c.command==='generate').args.request);
  expect(request.photos).toHaveLength(2);expect(request.fullCreature).toBe(true);
  expect(await page.locator('#result-image').evaluate(el=>el.naturalWidth)).toBeGreaterThan(0);
  report.checks.push('Two photos produce visible preview with no automatic payment');
  await page.locator('#keep').click();
  await expect(page.locator('#keep-dialog')).toBeVisible();
  await expect(page.locator('#confirm-price')).toContainText('6');
  await page.locator('#cancel-keep').click();
  expect(await page.evaluate(()=>window.testCalls.filter(c=>c.command==='keep_original').length)).toBe(0);
  await page.locator('#keep').click();await page.locator('#confirm-keep').click();
  await expect(page.locator('#keep-area')).toBeHidden();
  expect(await page.evaluate(()=>window.testCalls.filter(c=>c.command==='keep_original').length)).toBe(1);
  report.checks.push('Only confirmation of displayed price purchases original');
  await page.locator('#provider').selectOption('codex');await page.locator('#generate').click();
  await page.waitForFunction(()=>window.testCalls.some(c=>c.command==='get_job'&&c.args.jobId==='local-2'));
  await expect(page.locator('#result-image')).toBeVisible({timeout:20000});
  await expect(page.locator('#keep-area')).toBeHidden();
  report.checks.push('Codex result displays without Elyum purchase controls');
  await page.locator('nav [data-view=history]').click();
  await page.locator('.restore summary').click();
  await page.locator('#restore-id').fill('g_abcdef');
  await page.locator('#restore-form button').click();
  await expect(page.locator('#keep')).toBeVisible();
  expect(await page.evaluate(()=>window.testCalls.filter(c=>c.command==='keep_original').length)).toBe(1);
  report.checks.push('Remote job recovery displays preview without paying');
  await page.locator('#language').selectOption('ja');await page.reload();
  await expect(page.locator('html')).toHaveAttribute('lang','ja');
  report.checks.push('Language persists on restart');
  expect(errors).toEqual([]);
  report.ok=true;
} finally {
  await mkdir(join(root,'verification'),{recursive:true});
  await page.screenshot({path:join(root,'verification/frontend.png'),fullPage:true});
  await writeFile(join(root,'verification/frontend-tests.json'),JSON.stringify({...report,errors},null,2));
  await browser.close();await new Promise(r=>server.close(r));
}
console.log(JSON.stringify(report));
