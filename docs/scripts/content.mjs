import { readdir, readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';
import { site, pageUrl } from '../site.mjs';

export const docsRoot = fileURLToPath(new URL('../', import.meta.url));
export const contentRoot = `${docsRoot}src/content/docs/`;

export async function filesUnder(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const groups = await Promise.all(entries.map(async (entry) => {
    const path = `${directory}/${entry.name}`;
    return entry.isDirectory() ? filesUnder(path) : [path];
  }));
  return groups.flat().sort();
}

export function parsePage(source, slug) {
  const match = source.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n([\s\S]*)$/);
  if (!match) throw new Error(`Missing frontmatter: ${slug}`);
  const metadata = parse(match[1]);
  if (!metadata.title || !metadata.description) throw new Error(`Missing title/description: ${slug}`);
  return { slug, title: metadata.title, description: metadata.description, body: match[2].trim() };
}

export async function readPages() {
  const files = (await filesUnder(contentRoot)).filter((file) => file.endsWith('.md'));
  return Promise.all(files.map(async (file) => parsePage(
    await readFile(file, 'utf8'), file.slice(contentRoot.length).replace(/^\//, '').slice(0, -3),
  )));
}

export function exportMarkdown(page) {
  // Make site links useful when a Markdown file is read outside the website.
  const body = page.body.replace(/\]\((\/net_lang\/[^)]*)\)/g, (_, path) => `](${site}${path})`)
    .replace(/\]\((#[^)]*)\)/g, (_, hash) => `](${pageUrl(page.slug)}${hash})`);
  return `# ${page.title}\n\n${page.description}\n\nSource: ${pageUrl(page.slug)}\n\n${body}\n`;
}
