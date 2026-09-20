import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { docsRoot, contentRoot, readPages, exportMarkdown } from './content.mjs';
import { site, base, repository, pageUrl } from '../site.mjs';

const roadmap = (await readFile(new URL('../../ROADMAP.md', import.meta.url), 'utf8'))
  .replace(/^# .*\n/, '')
  .replace(/\]\((?!https?:|#)([^)]+)\)/g, (_, path) => `](${repository}/blob/main/${path})`);
await mkdir(`${contentRoot}project`, { recursive: true });
await writeFile(`${contentRoot}project/roadmap.md`, `---\ntitle: Roadmap\ndescription: Verified milestones, current work, and planned features.\neditUrl: ${repository}/edit/main/ROADMAP.md\n---\n\n${roadmap}`);

const pages = await readPages();
const publicRoot = `${docsRoot}public`;
await rm(`${publicRoot}/markdown`, { recursive: true, force: true });
await mkdir(`${publicRoot}/markdown`, { recursive: true });
const manifest = [];
for (const page of pages) {
  const path = `${publicRoot}/markdown/${page.slug}.md`;
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, exportMarkdown(page));
  manifest.push({ title: page.title, slug: page.slug, url: pageUrl(page.slug), markdown: `${site}${base}/markdown/${page.slug}.md` });
}
await writeFile(`${publicRoot}/markdown/index.json`, `${JSON.stringify(manifest, null, 2)}\n`);
await writeFile(`${publicRoot}/llms.txt`, `# Net-lang\n\n> Experimental network-oriented language with a working interpreter. Native compilation and cross-compilation remain planned.\n\nThe pages distinguish implemented behavior, limitations, and proposals. Do not treat proposed syntax as available.\n\n## Documentation\n\n${manifest.map((page) => `- [${page.title}](${page.markdown})`).join('\n')}\n\n## Complete reference\n\n- [All documentation](${site}${base}/llms-full.txt)\n- [Machine-readable index](${site}${base}/markdown/index.json)\n`);
await writeFile(`${publicRoot}/llms-full.txt`, `# Net-lang documentation\n\nGenerated from the same Markdown as the website. Planned features are not implemented APIs.\n\n${pages.map(exportMarkdown).join('\n---\n\n')}`);
console.log(`Generated roadmap and LLM exports for ${pages.length} pages.`);
