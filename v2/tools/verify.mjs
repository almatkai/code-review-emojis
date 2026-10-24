import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import vm from 'node:vm';
import ts from 'typescript';
import Ajv2020 from 'ajv/dist/2020.js';
import { captureOriginal } from './oracle.mjs';

const root = new URL('../', import.meta.url);
const read = path => readFileSync(new URL(path, root));
const json = path => JSON.parse(read(path));
const original = captureOriginal();
assert.deepEqual(original, json('tests/fixtures/upstream/capture.json'), 'capture must match executed pinned TSX');
const pkg = json('tests/fixtures/upstream/package.json');
assert.equal(pkg.author, 'rus64');
assert.equal(pkg.license, 'MIT');
assert.equal(original.sections[0].rows.length, 10);
assert.equal(original.sections[1].rows.length, 1);
assert.deepEqual([...original.sections[0].rows[6].icon].map(c => c.codePointAt(0)), [0x26cf, 0xfe0f]);
assert.deepEqual([...original.sections[0].rows[7].icon].map(c => c.codePointAt(0)), [0x267b, 0xfe0f]);
assert.deepEqual([...original.sections[0].rows[8].icon].map(c => c.codePointAt(0)), [0x1f3d5]);

const manifest = json('manifest.json');
const validate = new Ajv2020({ strict: false, allErrors: true }).compile(json('tests/fixtures/beta/extension-v2.schema.json'));
assert.ok(validate(manifest), JSON.stringify(validate.errors));
assert.equal(manifest.id, 'com.tezbar.native.code-review-emojis');
assert.equal(manifest.slug, 'code-review-emojis');
assert.equal(manifest.runtime, 'rust');
assert.deepEqual(manifest.replaces, ['raycast.code-review-emojis']);
assert.deepEqual(manifest.commands.map(c => c.name), ['index']);
assert.equal(manifest.commands[0].title, pkg.commands[0].title);
assert.equal(manifest.commands[0].runtime, 'rust');
assert.deepEqual(manifest.commands[0].platforms, ['macOS', 'Windows']);
assert.deepEqual(manifest.verification.map(v => [v.platform, v.state]), [['macOS', 'not-run'], ['Windows', 'not-run']]);
assert.equal(manifest.icon, '/extensions/icons/code-review-emojis.png');
assert.ok(!('author' in manifest));
assert.ok(!('preferences' in manifest));
const invalid = { ...manifest, author: 'rus64' };
assert.equal(validate(invalid), false, 'full beta schema must reject root author');

const provenance = json('provenance.json');
for (const group of ['upstream', 'beta']) {
  for (const entry of provenance[group].files) {
    assert.equal(createHash('sha256').update(read(entry.file)).digest('hex'), entry.sha256, entry.file);
  }
}
assert.equal(provenance.upstream.commit, '6734d85992290066fd31a7f5bee3cca3d3545a1a');
assert.equal(provenance.beta.commit, 'fd300e276356587726566026f072563eb9c34cb1');
assert.equal(provenance.packagingLicense, 'MIT AND Apache-2.0');
assert.ok(read('licenses/MIT-upstream.txt').equals(read('tests/fixtures/upstream/LICENSE')));
assert.ok(read('licenses/Apache-2.0-beta.txt').equals(read('tests/fixtures/beta/LICENSE')));
const icon = read('assets/code-review-emojis.png');
assert.equal(icon.length, 61058);
assert.equal(createHash('sha256').update(icon).digest('hex'), 'bd624e90c496cd28413add5990d3df7c31688425dd88aa8a7aef6ea205a8b41c');
assert.equal(icon.subarray(0, 8).toString('hex'), '89504e470d0a1a0a');

// Execute the actual beta parser/filter function ASTs, without React, DOM or
// desktop modules. Types are erased by TypeScript, function bodies are unchanged.
const parserSource = ts.createSourceFile('list-runtime.tsx', read('tests/fixtures/beta/list-runtime.tsx').toString(), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
const names = ['cleanSubtitle', 'parseRowIcon', 'parseAccessories', 'parseDetailFromChildren', 'parseListRows', 'groupBySection', 'clientSideFilter'];
const definitions = names.map(name => {
  const node = parserSource.statements.find(n => ts.isFunctionDeclaration(n) && n.name?.text === name);
  assert.ok(node, `Missing actual beta function ${name}`);
  return node.getText(parserSource);
}).join('\n');
const code = ts.transpileModule(definitions + `\nmodule.exports = { ${names.join(',')} };`, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText;
const module = { exports: {} };
vm.runInNewContext(code, { module }, { timeout: 1000 });
const { parseListRows, clientSideFilter, groupBySection } = module.exports;
const view = JSON.parse(execFileSync('cargo', ['run', '--quiet', '--locked', '--example', 'view'], { cwd: root, encoding: 'utf8' }));
assert.deepEqual(view.root.props, { searchBarPlaceholder: original.placeholder });
assert.equal(view.root.children.length, original.sections.length);
const rows = JSON.parse(JSON.stringify(parseListRows(view.root)));
const expectedRows = original.sections.flatMap(s => s.rows.map(r => ({ section: s.title, title: r.title, icon: { source: r.icon }, subtitle: '' })));
assert.equal(rows.length, expectedRows.length);
for (let i = 0; i < rows.length; i++) {
  assert.deepEqual({ section: rows[i].section, title: rows[i].title, icon: rows[i].icon, subtitle: rows[i].subtitle }, expectedRows[i]);
  assert.deepEqual(rows[i].actionIds, [view.actions[i].id]);
  assert.ok(!rows[i].copyText, 'Enter must invoke paste action');
  assert.equal(view.actions[i].title, i < 10 ? 'Paste' : 'Open in Browser');
  assert.equal(view.actions[i].kind, i < 10 ? 'action' : 'open');
  assert.ok(!('keywords' in view.root.children[i < 10 ? 0 : 1].children[i < 10 ? i : 0].props));
}
// Check the action-kind literals against the real beta shared type, rather
// than assuming a Raycast-only kind such as "paste" exists in this renderer.
const typeSource = ts.createSourceFile('extensionRuntime.ts', read('tests/fixtures/beta/extensionRuntime.ts').toString(), ts.ScriptTarget.Latest, true);
const actionType = typeSource.statements.find(n => ts.isTypeAliasDeclaration(n) && n.name.text === 'ExtensionRuntimeAction');
assert.ok(actionType && ts.isTypeLiteralNode(actionType.type));
const kindMember = actionType.type.members.find(n => n.name?.getText(typeSource) === 'kind');
assert.ok(kindMember && ts.isUnionTypeNode(kindMember.type));
const supportedKinds = kindMember.type.types.map(n => n.literal?.text);
for (const action of view.actions) assert.ok(supportedKinds.includes(action.kind), `Unsupported beta action kind ${action.kind}`);
assert.equal(view.actions.length, rows.length);
for (const query of ['', '  ', 'Emojis', 'MORE', 'ReFaCtOr', 'author', 'follow up', 'no-such-result', '🏕', '💭']) {
  const actual = clientSideFilter(rows, query).map(r => r.id);
  const q = query.trim().toLowerCase();
  const expected = rows.filter(r => !q || expectedRows[rows.indexOf(r)].title.toLowerCase().includes(q) || r.section.toLowerCase().includes(q)).map(r => r.id);
  assert.deepEqual(JSON.parse(JSON.stringify(actual)), expected, `client search: ${query}`);
}
assert.deepEqual(JSON.parse(JSON.stringify(groupBySection(rows))).map(g => [g.section, g.items.length]), [['Emojis', 10], ['More', 1]]);
console.log('PASS: executed pinned TSX oracle, full beta schema, provenance/licenses/icon, actual beta List parser, primary actions and client search');
