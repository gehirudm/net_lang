import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import { readFileSync } from 'node:fs';
import { site, base, repository } from './site.mjs';

const grammar = JSON.parse(readFileSync(new URL('../syntax/netlang.tmLanguage.json', import.meta.url), 'utf8'));

export default defineConfig({
  site,
  base,
  trailingSlash: 'always',
  integrations: [starlight({
    title: 'Net-lang',
    description: 'A language for network communication. Learn its syntax, interpreter, and runtime.',
    social: [{ icon: 'github', label: 'GitHub', href: repository }],
    editLink: { baseUrl: `${repository}/edit/main/docs/` },
    expressiveCode: { shiki: { langs: [{ ...grammar, name: 'netlang', aliases: ['net'] }] } },
    sidebar: [
      { label: 'Start here', items: ['index', 'getting-started', 'installation', 'cli'] },
      { label: 'Language', items: [{ autogenerate: { directory: 'language' } }] },
      { label: 'Networking', items: [{ autogenerate: { directory: 'networking' } }] },
      { label: 'Runtime', items: [{ autogenerate: { directory: 'runtime' } }] },
      { label: 'Compiler & project', items: [{ autogenerate: { directory: 'project' } }] },
    ],
  })],
});
