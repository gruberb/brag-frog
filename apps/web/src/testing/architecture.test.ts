import { describe, expect, it } from 'vitest';
import { ESLint } from 'eslint';
import { join } from 'node:path';

const root = join(__dirname, '..', '..');

// Proves the import/no-restricted-paths zones fire: each snippet is linted as if it lived in the given file.
async function lint(file: string, code: string) {
  const eslint = new ESLint({ cwd: root, overrideConfigFile: join(root, 'eslint.config.js') });
  const [res] = await eslint.lintText(code, { filePath: join(root, file) });
  return res.messages.filter((m) => m.ruleId === 'import/no-restricted-paths');
}

describe('import boundaries', () => {
  it('forbids a feature importing another feature', async () => {
    const msgs = await lint('src/features/logbook/components/x.ts', "import { useEntries } from '@/features/entries/api/entries';\nexport const a = useEntries;\n");
    expect(msgs).toHaveLength(1);
    expect(msgs[0].message).toMatch(/must not import from another feature/);
  });

  it('forbids a feature importing the app layer', async () => {
    const msgs = await lint('src/features/today/components/x.ts', "import { router } from '@/app/router';\nexport const a = router;\n");
    expect(msgs).toHaveLength(1);
  });

  it('forbids shared code importing features', async () => {
    const msgs = await lint('src/components/ui/x.ts', "import { useMe } from '@/features/auth/api/auth';\nexport const a = useMe;\n");
    expect(msgs).toHaveLength(1);
  });

  it('allows a feature importing shared code and itself', async () => {
    expect(await lint('src/features/entries/components/x.ts', "import { api } from '@/lib/api-client';\nimport { entryKeys } from '../api/entries';\nexport const a = [api, entryKeys];\n")).toHaveLength(0);
  });
});
