import { LANGUAGES, KEYS, translate, errorKey } from './i18n.js';

const $ = (id) => document.getElementById(id);
let language = LANGUAGES.includes(localStorage.getItem('language')) ? localStorage.getItem('language') : 'ru';
let mode = 'text', photos = [], reference = null, activeJob = null, jobs = [], polling = 0, generationBusy = false, keeping = false, confirmation = null;
const demo = new URLSearchParams(location.search).get('demo') === '1';
const t = (key, values) => translate(language, key, values);
const invoke = (command, args = {}) => window.__TAURI__?.core?.invoke ? window.__TAURI__.core.invoke(command, args) : Promise.reject('native_required');
const validImage = (url) => typeof url === 'string' && /^data:image\/(png|jpeg|webp);base64,[A-Za-z0-9+/=\r\n]+$/.test(url);
const formatCost = (cost) => new Intl.NumberFormat(language, { maximumFractionDigits: 3 }).format(cost);
const knownPrice = (cost) => typeof cost === 'number' && Number.isFinite(cost) && cost >= 0;

function message(id, key, values = {}, error = false) {
  const el = $(id);
  el.textContent = key ? t(key, values) : '';
  el.hidden = !key;
  el.dataset.messageKey = key || '';
  el.dataset.messageValues = JSON.stringify(values);
  el.classList.toggle('error', error);
}
function showError(error, id = 'form-error') { message(id, errorKey(error), {}, true); }
function setBusy(el, value) { el.disabled = value; el.setAttribute('aria-busy', String(value)); }

export function setLanguage(value, persist = true) {
  language = LANGUAGES.includes(value) ? value : 'ru';
  if (persist) localStorage.setItem('language', language);
  document.documentElement.lang = language;
  document.title = t('appName');
  $('language').value = language;
  document.querySelectorAll('[data-t]').forEach((el) => el.textContent = t(el.dataset.t));
  document.querySelectorAll('[data-message-key]').forEach((el) => { if (el.dataset.messageKey) el.textContent = t(el.dataset.messageKey, JSON.parse(el.dataset.messageValues || '{}')); });
  document.querySelector('.main-nav').setAttribute('aria-label', t('navigation'));
  document.querySelector('.mode-tabs').setAttribute('aria-label', t('mode'));
  $('description').placeholder = t('promptPlaceholder');
  $('result-image').alt = t('resultAlt');
  document.querySelector('#result-empty img').alt = t('trioAlt');
  const ideas = $('idea-buttons'); ideas.replaceChildren();
  for (let i = 1; i <= 3; i++) {
    const b = document.createElement('button'); b.type = 'button'; b.textContent = t(`idea${i}`);
    b.addEventListener('click', () => { $('description').value = t(`ideaPrompt${i}`); $('description').removeAttribute('aria-invalid'); message('form-error', null); $('description').focus(); });
    ideas.append(b);
  }
  updateProvider(); updatePhotoStyle(); renderPhotos(); renderHistory(); renderJob();
  if (generationBusy) $('generate').querySelector('span').textContent = t('submitting');
}

function setView(view, updateHash = true) {
  const actual = ['create', 'history', 'settings'].includes(view) ? view : 'create';
  document.querySelectorAll('.view').forEach((el) => el.hidden = el.id !== `view-${actual}`);
  document.querySelectorAll('.main-nav [data-view]').forEach((el) => { if (el.dataset.view === actual) el.setAttribute('aria-current', 'page'); else el.removeAttribute('aria-current'); });
  if (updateHash && location.hash !== `#${actual}`) history.replaceState(null, '', `#${actual}`);
}
function setMode(value) {
  mode = value === 'photo' ? 'photo' : 'text';
  $('photo-inputs').hidden = mode !== 'photo';
  document.querySelectorAll('[data-mode]').forEach((el) => { const selected = el.dataset.mode === mode; el.setAttribute('aria-selected', String(selected)); el.tabIndex = selected ? 0 : -1; });
  $('mode-panel').setAttribute('aria-labelledby', `mode-${mode}`);
  message('form-error', null);
}
function updateProvider() {
  const codex = $('provider').value === 'codex';
  $('elyum-options').hidden = codex;
  $('codex-options').hidden = !codex;
  $('credit-mode-wrap').hidden = codex;
  $('billing-help').textContent = t(codex ? 'codexBilling' : 'billingHelp');
}
function updatePhotoStyle() {
  $('photo-style-help').textContent = t(document.querySelector('[name="photoStyle"]:checked').value === 'creature' ? 'creatureHelp' : 'classicHelp');
}
function readPhoto(file) {
  if (!['image/png', 'image/jpeg', 'image/webp'].includes(file.type) || file.size > 20 * 1024 * 1024 || file.size === 0) return Promise.reject('image_invalid');
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject('image_invalid');
    reader.onload = () => {
      const img = new Image();
      img.onerror = () => reject('image_invalid');
      img.onload = () => img.naturalWidth * img.naturalHeight > 40000000 ? reject('image_invalid') : resolve({ name: file.name, data: reader.result });
      img.src = reader.result;
    };
    reader.readAsDataURL(file);
  });
}
async function addPhotos(files) {
  if (photos.length + files.length > 8) { showError('photos_limit'); return; }
  try { const added = await Promise.all([...files].map(readPhoto)); if (photos.length + added.length > 8) throw 'photos_limit'; photos.push(...added); renderPhotos(); message('form-error', null); }
  catch (e) { showError(e); }
  $('photos').value = '';
}
function renderPhotos() {
  const list = $('photo-thumbnails'); list.replaceChildren();
  photos.forEach((photo, index) => {
    const fig = document.createElement('figure'); fig.className = 'photo-thumbnail';
    const img = document.createElement('img'); img.src = photo.data; img.alt = t('photoAlt', { number: index + 1 }); img.width = 80; img.height = 80;
    const b = document.createElement('button'); b.type = 'button'; b.textContent = '×'; b.setAttribute('aria-label', t('removePhoto', { number: index + 1 }));
    b.addEventListener('click', () => { photos.splice(index, 1); renderPhotos(); });
    fig.append(img, b); list.append(fig);
  });
}

function rememberJob(job) {
  const old = jobs.findIndex((j) => j.id === job.id);
  if (old >= 0) jobs[old] = { ...jobs[old], ...job }; else jobs.unshift(job);
  jobs.sort((a, b) => String(b.createdAt).localeCompare(String(a.createdAt)));
  renderHistory();
}
function renderJob() {
  const job = activeJob;
  const hasImage = job && validImage(job.imageUrl);
  const running = job && ['queued', 'running'].includes(job.status);
  $('result-empty').hidden = !!hasImage || !!running;
  $('result-image').hidden = !hasImage;
  $('result-loading').hidden = !running || !!hasImage;
  $('result-stage').classList.toggle('has-image', !!hasImage);
  if (hasImage && $('result-image').src !== job.imageUrl) $('result-image').src = job.imageUrl;
  $('result-state').textContent = t(job?.status || 'ready');
  $('keep-area').hidden = job?.status !== 'preview';
  $('keep-explanation').textContent = knownPrice(job?.cost) ? t('keepPrice', { cost: formatCost(job.cost) }) : t('unknownPrice');
  // A charge can only be confirmed after the actual preview has loaded.
  const renderedPreview = hasImage && $('result-image').complete && $('result-image').naturalWidth > 0;
  $('keep').disabled = keeping || !knownPrice(job?.cost) || !renderedPreview;
  $('keep').textContent = t(keeping ? 'keeping' : 'keep');
  $('result-actions').hidden = !job;
  $('export').disabled = !hasImage || keeping;
  $('export').textContent = t(job?.status === 'preview' ? 'exportPreview' : 'export');
  $('refresh-job').disabled = keeping;
  $('job-id').textContent = job?.id || '';
  if (job?.error) showError(job.error, 'result-message');
}
function selectJob(job, poll = true) {
  polling++;
  activeJob = job; rememberJob(job); message('result-message', null); renderJob();
  if (poll && ['queued', 'running'].includes(job.status)) pollJob(job.id, polling);
}
async function pollJob(id, token) {
  for (let attempt = 0; attempt < 120 && token === polling; attempt++) {
    try {
      const job = await invoke('get_job', { jobId: id });
      rememberJob(job);
      if (token !== polling) return;
      activeJob = job; renderJob();
      if (!['queued', 'running'].includes(job.status)) return;
    } catch (e) { if (token === polling) showError(e, 'result-message'); return; }
    await new Promise((resolve) => setTimeout(resolve, 2000));
  }
  if (token === polling) message('result-message', 'waitHelp');
}
async function refreshJob() {
  if (!activeJob || keeping) return;
  const id = activeJob.id; const token = ++polling;
  setBusy($('refresh-job'), true); message('result-message', 'loading');
  try { const job = await invoke('get_job', { jobId: id }); rememberJob(job); if (token === polling) selectJob(job); }
  catch (e) { showError(e, 'result-message'); }
  finally { setBusy($('refresh-job'), false); }
}
async function generate(event) {
  event.preventDefault();
  if (generationBusy || keeping) return;
  message('form-error', null);
  if (mode === 'photo' && !photos.length) { showError('photos_required'); $('photos').focus(); return; }
  if (mode === 'text' && !$('description').value.trim()) { showError('description_required'); $('description').setAttribute('aria-invalid', 'true'); $('description').focus(); return; }
  const request = { provider: $('provider').value, mode, description: $('description').value.trim(), photos: mode === 'photo' ? photos.map((p) => p.data) : [], customReference: reference?.data || null, character: $('character').value, background: $('background').value, emotion: $('emotion').value, strength: Number($('strength').value), fullCreature: document.querySelector('[name="photoStyle"]:checked').value === 'creature', aspectRatio: $('aspectRatio').value, resolution: $('resolution').value, creditMode: $('creditMode').checked, cleanCodex: $('cleanCodex').checked };
  generationBusy = true; setBusy($('generate'), true); $('generate').querySelector('span').textContent = t('submitting');
  try { const job = await invoke('generate', { request }); selectJob(job); }
  catch (e) { showError(e); }
  finally { generationBusy = false; setBusy($('generate'), false); $('generate').querySelector('span').textContent = t('generate'); }
}
function requestKeep() {
  if (!activeJob || activeJob.status !== 'preview' || !knownPrice(activeJob.cost) || keeping || $('keep').disabled) return;
  confirmation = { jobId: activeJob.id, confirmedCost: activeJob.cost };
  $('confirm-price').textContent = t('keepPrice', { cost: formatCost(activeJob.cost) });
  $('keep-dialog').showModal();
}
async function confirmKeep() {
  if (!confirmation || keeping) return;
  const confirmed = { ...confirmation }; confirmation = null;
  $('keep-dialog').close(); keeping = true; polling++; renderJob(); message('result-message', 'keeping');
  setBusy($('generate'), true); setBusy($('confirm-keep'), true);
  try { const job = await invoke('keep_original', confirmed); selectJob(job, false); }
  catch (e) {
    // Resolve an uncertain response through a read. Never resend a paid write.
    try { const job = await invoke('get_job', { jobId: confirmed.jobId }); selectJob(job, false); } catch { /* keep the recoverable previous state */ }
    if (activeJob?.status !== 'completed') showError(e, 'result-message');
  } finally { keeping = false; renderJob(); setBusy($('generate'), generationBusy); setBusy($('confirm-keep'), false); }
}
function renderHistory() {
  const list = $('history-list'); list.replaceChildren();
  if (!jobs.length) { message('history-status', 'historyEmpty'); return; }
  message('history-status', null);
  for (const job of jobs) {
    const button = document.createElement('button'); button.type = 'button'; button.className = 'history-item';
    if (validImage(job.imageUrl)) { const img = document.createElement('img'); img.src = job.imageUrl; img.width = 320; img.height = 200; img.alt = job.description || t('resultAlt'); button.append(img); }
    else { const p = document.createElement('div'); p.className = 'history-placeholder'; p.textContent = t(job.status); button.append(p); }
    const title = document.createElement('p'); title.textContent = job.description || t('result');
    const meta = document.createElement('p'); meta.textContent = `${t(job.status)} (${job.provider === 'codex' ? 'Codex' : 'Elyum'})`;
    const date = document.createElement('time'); date.dateTime = job.createdAt || ''; const parsed = new Date(job.createdAt); date.textContent = Number.isNaN(parsed.getTime()) ? '' : parsed.toLocaleString(language, { dateStyle: 'medium', timeStyle: 'short' });
    button.append(title, meta, date); button.addEventListener('click', async () => { setView('create'); selectJob(job, false); await refreshJob(); });
    list.append(button);
  }
}
async function loadHistory() {
  setBusy($('refresh-history'), true); message('history-status', 'loading');
  try { const values = await invoke('list_jobs'); jobs = values.map((job) => ({ ...jobs.find((j) => j.id === job.id), ...job })); renderHistory(); }
  catch (e) { showError(e, 'history-status'); }
  finally { setBusy($('refresh-history'), false); }
}
async function codexLogin() {
  const buttons = [$('codex-login'), $('codex-login-inline')]; buttons.forEach((el) => setBusy(el, true));
  message('codex-status', 'loading'); setView('settings');
  try { await invoke('codex_login'); message('codex-status', 'loginStarted'); }
  catch (e) { showError(e, 'codex-status'); }
  finally { buttons.forEach((el) => setBusy(el, false)); }
}

$('language').addEventListener('change', (e) => setLanguage(e.target.value));
document.querySelectorAll('[data-view]').forEach((el) => el.addEventListener('click', () => setView(el.dataset.view)));
window.addEventListener('hashchange', () => setView(location.hash.slice(1), false));
document.querySelectorAll('[data-mode]').forEach((el) => { el.addEventListener('click', () => setMode(el.dataset.mode)); el.addEventListener('keydown', (e) => { if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key)) { e.preventDefault(); setMode(e.key === 'Home' ? 'text' : e.key === 'End' ? 'photo' : mode === 'photo' ? 'text' : 'photo'); $(`mode-${mode}`).focus(); } }); });
document.querySelectorAll('[name="photoStyle"]').forEach((el) => el.addEventListener('change', updatePhotoStyle));
$('provider').addEventListener('change', updateProvider);
$('description').addEventListener('input', () => $('description').removeAttribute('aria-invalid'));
$('strength').addEventListener('input', () => $('strength-value').textContent = `${$('strength').value}%`);
$('photos').addEventListener('change', (e) => addPhotos(e.target.files));
for (const event of ['dragenter', 'dragover']) $('photo-drop').addEventListener(event, (e) => { e.preventDefault(); $('photo-drop').classList.add('dragging'); });
for (const event of ['dragleave', 'drop']) $('photo-drop').addEventListener(event, (e) => { e.preventDefault(); $('photo-drop').classList.remove('dragging'); });
$('photo-drop').addEventListener('drop', (e) => addPhotos(e.dataTransfer.files));
$('customReference').addEventListener('change', async (e) => { try { reference = e.target.files.length ? await readPhoto(e.target.files[0]) : null; $('remove-reference').hidden = !reference; $('reference-name').textContent = reference?.name || ''; $('reference-name').hidden = !reference; } catch (err) { reference = null; e.target.value = ''; $('remove-reference').hidden = true; $('reference-name').hidden = true; showError(err); } });
$('remove-reference').addEventListener('click', () => { reference = null; $('customReference').value = ''; $('remove-reference').hidden = true; $('reference-name').hidden = true; });
$('create-form').addEventListener('submit', generate);
$('result-image').addEventListener('load', renderJob);
$('result-image').addEventListener('error', () => { $('keep').disabled = true; showError('download_failed', 'result-message'); });
$('keep').addEventListener('click', requestKeep);
$('confirm-keep').addEventListener('click', confirmKeep);
$('cancel-keep').addEventListener('click', () => { confirmation = null; $('keep-dialog').close(); });
$('keep-dialog').addEventListener('cancel', () => { confirmation = null; });
$('keep-dialog').addEventListener('click', (e) => { if (e.target === $('keep-dialog')) { const r = e.target.getBoundingClientRect(); if (e.clientX < r.left || e.clientX > r.right || e.clientY < r.top || e.clientY > r.bottom) { confirmation = null; e.target.close(); } } });
$('refresh-job').addEventListener('click', refreshJob);
$('refresh-history').addEventListener('click', loadHistory);
$('export').addEventListener('click', async () => { if (!activeJob) return; setBusy($('export'), true); try { const path = await invoke('export_image', { jobId: activeJob.id }); if (path) message('result-message', 'saved', { path }); } catch (e) { showError(e, 'result-message'); } finally { setBusy($('export'), false); } });
$('restore-form').addEventListener('submit', async (e) => { e.preventDefault(); const b = e.target.querySelector('button'); setBusy(b, true); message('restore-status', 'loading'); try { const job = await invoke('restore_job', { jobId: $('restore-id').value.trim() }); selectJob(job); setView('create'); message('restore-status', null); } catch (err) { showError(err, 'restore-status'); } finally { setBusy(b, false); } });
$('codex-login').addEventListener('click', codexLogin); $('codex-login-inline').addEventListener('click', codexLogin);
$('check-codex').addEventListener('click', async () => { setBusy($('check-codex'), true); message('codex-status', 'loading'); try { const result = await invoke('codex_account'); message('codex-status', result.authenticated ? 'loggedIn' : 'loggedOut'); } catch (e) { showError(e, 'codex-status'); } finally { setBusy($('check-codex'), false); } });
$('check-account').addEventListener('click', async () => { setBusy($('check-account'), true); message('account-status', 'loading'); try { const result = await invoke('account'); const balance = result.balance ?? result.credits ?? result.account?.balance; message('account-status', knownPrice(balance) ? 'balance' : 'connected', { cost: formatCost(knownPrice(balance) ? balance : 0) }); } catch (e) { showError(e, 'account-status'); } finally { setBusy($('check-account'), false); } });

async function smokeTest() {
  const checks = { bridge: !!window.__TAURI__?.core?.invoke, form: !!$('create-form'), preview: !!$('result-stage'), confirmation: !!$('keep-dialog'), noAutoKeep: $('keep-area').hidden, translations: true, providerSwitch: true, font: false, icon: false };
  for (const lang of LANGUAGES) { setLanguage(lang, false); checks.translations &&= document.documentElement.lang === lang && $('generate').textContent.trim() === translate(lang, 'generate') && KEYS.every((key) => translate(lang, key) !== key); }
  for (const provider of ['codex', 'elyum', 'codex', 'elyum']) { $('provider').value = provider; updateProvider(); checks.providerSwitch &&= $('elyum-options').hidden === (provider === 'codex') && $('codex-options').hidden === (provider !== 'codex'); }
  await document.fonts.load('24px ChiikawaKawaii'); checks.font = document.fonts.check('24px ChiikawaKawaii');
  const icon = document.querySelector('.brand img'); if (!icon.complete) await new Promise((r) => { icon.onload = r; icon.onerror = r; }); checks.icon = icon.naturalWidth > 0;
  setLanguage('ru', false); setView('create', false);
  await invoke('smoke_test_complete', { report: { ok: Object.values(checks).every(Boolean), checks } });
}
setLanguage(language, false); setView(location.hash.slice(1), false);
$('demo-banner').hidden = !demo;
if (demo) { $('output-directory').textContent = 'Demo'; $('app-version').textContent = '3.0.1'; }
else {
  invoke('app_info').then(async (info) => { $('output-directory').textContent = info.outputDir; $('app-version').textContent = info.version; if (info.smokeTest) { await smokeTest(); return; } await loadHistory(); }).catch((e) => { showError(e); });
}
