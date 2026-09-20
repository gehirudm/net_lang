import test from 'node:test';
import assert from 'node:assert/strict';
import { exportMarkdown, parsePage } from './content.mjs';

test('exports preserve proposals and code while resolving site links', () => {
  const page = parsePage('---\ntitle: Proposal\ndescription: Not implemented.\n---\n[Syntax](/net_lang/language/syntax/)\n\n[Local](#future)\n\n## Future\n\n```netlang\nWS "wss://example.com";\n```', 'project/proposal');
  const result = exportMarkdown(page);
  assert.ok(result.includes('Not implemented.'));
  assert.ok(result.includes('https://gehirudm.github.io/net_lang/language/syntax/'));
  assert.ok(result.includes('https://gehirudm.github.io/net_lang/project/proposal/#future'));
  assert.ok(result.includes('WS "wss://example.com";'));
  assert.ok(!result.includes('title: Proposal'));
  assert.equal(result, exportMarkdown(page));
});

test('missing metadata fails instead of silently omitting a page', () => {
  assert.throws(() => parsePage('plain text', 'bad'), /frontmatter/);
  assert.throws(() => parsePage('---\ntitle: Missing\n---\ntext', 'bad'), /description/);
});
