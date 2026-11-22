import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import { absoluteUrl, prepareDescription } from '../frontend/src/lib/description.ts';
import { disabledName, enabledName, installedName, parentAndName } from '../frontend/src/lib/files.ts';

describe('mod files', () => {
  test('disabling appends .disabled, enabling strips it in any case', () => {
    assert.equal(disabledName('carryon_1.8.0.zip'), 'carryon_1.8.0.zip.disabled');
    assert.equal(enabledName('carryon_1.8.0.zip.disabled'), 'carryon_1.8.0.zip');
    assert.equal(enabledName('carryon_1.8.0.zip.DISABLED'), 'carryon_1.8.0.zip');
    assert.equal(enabledName('carryon_1.8.0.zip'), 'carryon_1.8.0.zip');
  });

  test('an update keeps the enabled state of the file it replaces', () => {
    assert.equal(installedName('carryon_1.9.0.zip', true), 'carryon_1.9.0.zip');
    assert.equal(installedName('carryon_1.9.0.zip', null), 'carryon_1.9.0.zip');
    assert.equal(installedName('carryon_1.9.0.zip', false), 'carryon_1.9.0.zip.disabled');
  });

  test('folder paths split into a file endpoint root and a name', () => {
    assert.deepEqual(parentAndName('data/Mods'), ['/data', 'Mods']);
    assert.deepEqual(parentAndName('Mods'), ['/', 'Mods']);
    assert.deepEqual(parentAndName('srv/data/Mods'), ['/srv/data', 'Mods']);
  });
});

describe('ModDB descriptions', () => {
  test('blank lines are dropped so Markdown never sees an indented code block', () => {
    const prepared = prepareDescription('<p>One</p>\r\n\r\n    <p>Two</p>\n\n');
    assert.equal(prepared, '<div>\n<p>One</p>\n    <p>Two</p>\n</div>');
  });

  test('relative and protocol-relative links point at the ModDB; other schemes are dropped', () => {
    assert.equal(absoluteUrl('/files/asset/1/logo.png'), 'https://mods.vintagestory.at/files/asset/1/logo.png');
    assert.equal(absoluteUrl('//cdn.example.com/a.png'), 'https://cdn.example.com/a.png');
    assert.equal(absoluteUrl(' https://example.com '), 'https://example.com');
    assert.equal(absoluteUrl('mailto:a@b.c'), 'mailto:a@b.c');
    assert.equal(absoluteUrl('javascript:alert(1)'), undefined);
    assert.equal(absoluteUrl('data:text/html,x'), undefined);
    assert.equal(absoluteUrl(''), undefined);
    assert.equal(absoluteUrl(null), undefined);
  });
});
