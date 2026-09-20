import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHighlighter } from 'shiki';

const grammar = JSON.parse(await readFile(new URL('../../syntax/netlang.tmLanguage.json', import.meta.url), 'utf8'));
const highlighter = await createHighlighter({ themes: ['github-dark'], langs: [{ ...grammar, name: 'netlang' }] });
function scopes(source) {
  return highlighter.codeToTokens(source, { lang: 'netlang', theme: 'github-dark', includeExplanation: 'scopeName' }).tokens.flat()
    .flatMap((token) => token.explanation || []).map((entry) => ({ text: entry.content, scopes: entry.scopes.map((scope) => scope.scopeName) }));
}
function has(entries, text, scope) {
  return entries.some((entry) => entry.text.includes(text) && entry.scopes.includes(scope));
}

test('network operators, durations, annotations and function calls are highlighted', () => {
  const entries = scopes('let conn = TCP "host:9000"; conn SEND data; let packet = conn RECEIVE; socket SEND packet TO address; let wait = 50ms; fn f(x: int) -> string { return decode_utf8(x); }');
  for (const word of ['TCP', 'SEND', 'RECEIVE', 'TO']) assert.ok(has(entries, word, 'keyword.other.network.netlang'), word);
  assert.ok(has(entries, '50ms', 'constant.numeric.duration.netlang'));
  assert.ok(has(entries, 'int', 'storage.type.netlang'));
  assert.ok(has(entries, 'decode_utf8', 'entity.name.function.netlang'));
});

test('strings, escapes, multiline comments, and keyword boundaries remain distinct', () => {
  const entries = scopes('let GETaway = "https://host/${id}\\n"; /* SEND\nRECEIVE */ GET url; // TCP');
  assert.ok(!has(entries, 'GETaway', 'keyword.other.network.netlang'));
  assert.ok(has(entries, 'https://host/${id}', 'string.quoted.double.netlang'));
  assert.ok(has(entries, '\\n', 'constant.character.escape.netlang'));
  assert.ok(has(entries, 'RECEIVE', 'comment.block.netlang'));
  assert.ok(has(entries, 'TCP', 'comment.line.double-slash.netlang'));
  assert.ok(has(entries, 'GET', 'keyword.other.network.netlang'));
});

test('every reserved word in the Flex lexer has a keyword or literal scope', async () => {
  const lexer = await readFile(new URL('../../lexer/netlang.l', import.meta.url), 'utf8');
  const words = [...lexer.matchAll(/^"([a-zA-Z][a-zA-Z0-9_]*)" return NET_\w+;/gm)].map((match) => match[1]);
  assert.ok(words.length > 20, 'Lexer keyword extraction failed');
  for (const word of words) {
    const entries = scopes(word);
    assert.ok(entries.some((entry) => entry.text === word && entry.scopes.some((scope) =>
      scope.startsWith('keyword.') || scope.startsWith('constant.language.'))), word);
  }
});

test.after(() => highlighter.dispose());
