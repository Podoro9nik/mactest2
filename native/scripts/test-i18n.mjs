import assert from 'node:assert/strict';
import test from 'node:test';
import { errorKey, LANGUAGES, translate } from '../ui/i18n.js';

const providerErrors = ['provider_error', 'provider_response'];
const uploadErrors = ['upload_too_large', 'upload_failed', 'upload_response'];

test('provider and upload failures are not disguised as connection failures', () => {
  for (const code of [...providerErrors, ...uploadErrors]) {
    const expected = `error_${code}`;
    assert.equal(errorKey(code), expected);
    assert.equal(errorKey({ code }), expected);
    assert.equal(errorKey(new Error(code)), expected);
    for (const language of LANGUAGES) {
      const message = translate(language, expected);
      assert.notEqual(message, translate(language, 'error_network'));
      assert.notEqual(message, translate(language, 'errorGeneric'));
      assert.notEqual(message, expected, `Missing ${language} translation for ${code}`);
    }
  }
  assert.equal(errorKey('network'), 'error_network');
});

test('upload failures explain that generation has not started in all three languages', () => {
  const notStarted = {
    ru: 'Генерация не запускалась.',
    en: 'Generation has not started.',
    ja: '生成は開始されていません。',
  };
  for (const code of uploadErrors) {
    const localized = LANGUAGES.map(language => translate(language, errorKey(code)));
    assert.equal(new Set(localized).size, 3, `${code} must have three translations`);
    for (const language of LANGUAGES) {
      const message = translate(language, errorKey(code));
      assert.ok(message.includes(notStarted[language]), `${language}: ${code}`);
      assert.ok(!message.includes(translate(language, 'refresh')),
        'A pre-submission upload failure must not ask to refresh a nonexistent task');
    }
  }
});

test('provider diagnostics cannot leak into displayed errors or change payment recovery', () => {
  const diagnostic = 'private-provider-diagnostic';
  for (const code of [...providerErrors, ...uploadErrors]) {
    for (const language of LANGUAGES) {
      const message = translate(language, errorKey(`${code}: ${diagnostic}`));
      assert.ok(!message.includes(diagnostic));
    }
  }
  assert.equal(errorKey('submit_uncertain'), 'error_submit_uncertain');
  assert.equal(errorKey('keep_uncertain'), 'error_keep_uncertain');
  assert.equal(errorKey('codex_error'), 'error_codex_image_unavailable');
});
