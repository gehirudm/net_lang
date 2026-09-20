import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { execFileSync } from 'node:child_process';
import { docsRoot, readPages } from './content.mjs';

const binary = process.env.NETLANG_BIN || resolve(docsRoot, '../target/debug/netlang');
const directory = await mkdtemp(join(tmpdir(), 'netlang-docs-'));
let count = 0;
try {
  for (const page of await readPages()) {
    // Only explicitly annotated complete programs execute; network examples use check.
    for (const match of page.body.matchAll(/^```netlang (run|check|parse)\s*\n([\s\S]*?)^```/gm)) {
      const file = join(directory, `example-${++count}.net`);
      await writeFile(file, match[2]);
      try {
        execFileSync(binary, [match[1] === 'parse' ? 'ast' : match[1], file], { timeout: 10000, stdio: 'pipe' });
      } catch (error) {
        throw new Error(`${page.slug} example ${count} failed:\n${error.stderr || error.message}`);
      }
    }
  }
  if (count === 0) throw new Error('No complete documentation examples found.');
  console.log(`Verified ${count} complete examples against ${binary}.`);
} finally {
  await rm(directory, { recursive: true, force: true });
}
