# Net-lang documentation website

Astro Starlight builds the public guides from `src/content/docs/`. The historical
technical reference remains in `../docs-internal/LANGUAGE_DESIGN.md`; maintain
the public guides for future changes. The root `ROADMAP.md` remains authoritative
for milestone tracking and is copied into the website during generation.

## Local development

Use Node.js 22.19 or newer (Node 22 is used in CI) and npm. From this directory:

```sh
npm ci
npm run dev
```

Open `http://localhost:4321/net_lang/`. The site/base configuration is centralized
in `site.mjs`. Build and validate with:

```sh
npm run check
npm test
npm run build
npm run verify
```

From the repository root, build the compiler with `cargo build --locked`, then
run `npm run examples --prefix docs`. `NETLANG_BIN` can select another compiled
binary. Complete examples are marked with `netlang run`, `netlang check`, or
`netlang parse` fence metadata. Only `run` examples execute; keep those offline.
Fragments and proposals use plain `netlang` fences and are not executed.

## Content and generated files

Every page needs `title` and `description` frontmatter. Identify proposals and
limitations explicitly. Use `/net_lang/` for internal site links. `verify` checks
the built pages' local links, anchors, assets, search index, and export coverage.
External links are not crawled by CI.

`npm run generate` produces the roadmap page, `public/llms.txt`,
`public/llms-full.txt`, and `public/markdown/` (including a JSON index). These files
are ignored and regenerated for development, checks, and builds. Markdown exports
preserve code and status labels and expand site-relative links. There is no
separate manually maintained LLM reference.

The shared grammar in `../syntax/netlang.tmLanguage.json` is loaded by Starlight's
Expressive Code/Shiki integration. Grammar tests exercise token scopes, including
comments and strings. The future VS Code extension can reuse this same file.

## Deployment

`.github/workflows/docs.yml` validates pull requests and builds/deploys `main` to
GitHub Pages. In repository Settings → Pages, select **GitHub Actions** as the
build source. The configured destination is
`https://gehirudm.github.io/net_lang/`; configuration alone does not establish that
the site is live. Deployment requires a successful hosted workflow and the
repository's Pages/environment permissions. The workflow never deploys PR builds.

Local compiler builds remain independent of Node.js and the documentation site.

Configuration references: [Starlight manual setup](https://starlight.astro.build/manual-setup/),
[Expressive Code configuration](https://expressive-code.com/reference/configuration/),
and [Astro on GitHub Pages](https://docs.astro.build/en/guides/deploy/github/).
