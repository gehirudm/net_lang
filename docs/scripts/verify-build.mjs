import assert from 'node:assert/strict';
import { readFile, stat } from 'node:fs/promises';
import { join } from 'node:path';
import { parse } from 'parse5';
import { docsRoot, filesUnder, readPages } from './content.mjs';
import { site, base } from '../site.mjs';

const dist = `${docsRoot}dist`;
const files = await filesUnder(dist);
const htmlFiles = files.filter((file) => file.endsWith('.html'));
assert.ok(htmlFiles.length > 0, 'No built HTML');
const documents = new Map();
for (const file of htmlFiles) {
  const ids = new Set();
  const links = [];
  function visit(node) {
    // Starlight's generated 404.html has a /404/ canonical URL; Pages serves it
    // as the error document, not as a directory route. Still check its UI links.
    const errorCanonical = file.endsWith('/404.html') && node.tagName === 'link'
      && node.attrs?.some((attr) => attr.name === 'rel' && attr.value === 'canonical');
    for (const attr of node.attrs || []) {
      if (attr.name === 'id') ids.add(attr.value);
      if (!errorCanonical && (attr.name === 'href' || attr.name === 'src')) links.push(attr.value);
    }
    for (const child of node.childNodes || []) visit(child);
  }
  visit(parse(await readFile(file, 'utf8')));
  documents.set(file, { ids, links });
}
let count = 0;
for (const [file, { links }] of documents) {
  const source = `${site}${base}/${file.slice(dist.length + 1).replace(/index\.html$/, '')}`;
  for (const link of links) {
    const url = new URL(link, source);
    if (url.origin !== site) continue;
    assert.ok(url.pathname.startsWith(`${base}/`), `Link escapes deployment base: ${link} in ${file}`);
    let target = join(dist, decodeURIComponent(url.pathname.slice(base.length)));
    let info;
    try { info = await stat(target); } catch { throw new Error(`Broken link ${link} in ${file}`); }
    if (info.isDirectory()) target = join(target, 'index.html');
    assert.ok((await stat(target)).isFile(), `Missing file: ${target}`);
    if (url.hash && documents.has(target)) {
      assert.ok(documents.get(target).ids.has(decodeURIComponent(url.hash.slice(1))), `Missing anchor ${link} in ${file}`);
    }
    count++;
  }
}
const pages = await readPages();
const manifest = JSON.parse(await readFile(`${dist}/markdown/index.json`, 'utf8'));
assert.equal(manifest.length, pages.length);
const full = await readFile(`${dist}/llms-full.txt`, 'utf8');
for (const page of pages) {
  assert.ok(full.includes(`# ${page.title}\n`), `Missing export: ${page.slug}`);
  await stat(`${dist}/markdown/${page.slug}.md`);
}
assert.ok(files.some((file) => file.includes('/pagefind/') && file.endsWith('.js')), 'Search index missing');
assert.ok((await readFile(`${dist}/index.html`, 'utf8')).includes('data-language="netlang"'), 'Net-lang code block missing');
console.log(`Verified ${htmlFiles.length} HTML pages, ${count} local links/assets, and ${pages.length} Markdown exports.`);
