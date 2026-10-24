import ts from 'typescript';
import vm from 'node:vm';
import { readFileSync } from 'node:fs';

const fixture = new URL('../tests/fixtures/upstream/index.tsx', import.meta.url);
export function captureOriginal() {
  const element = (type, props, ...children) => ({
    type, props: props || {}, children: children.flat(Infinity).filter(Boolean),
  });
  // Assigning properties to string primitives is not possible; use component
  // markers and normalize them when capturing the executed element tree.
  function list() {}
  list.Section = 'List.Section';
  list.Item = 'List.Item';
  const action = { Paste: 'Action.Paste', OpenInBrowser: 'Action.OpenInBrowser' };
  const module = { exports: {} };
  const code = ts.transpileModule(readFileSync(fixture, 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.React, esModuleInterop: true },
  }).outputText;
  vm.runInNewContext(code, {
    module, exports: module.exports,
    require(name) {
      if (name === 'react') return { createElement: element };
      if (name === '@raycast/api') return { List: list, Action: action, ActionPanel: 'ActionPanel' };
      throw new Error(`Unexpected import ${name}`);
    },
  }, { filename: 'pinned-original.cjs', timeout: 1000 });
  const tree = module.exports.default();
  return {
    placeholder: tree.props.searchBarPlaceholder,
    sections: tree.children.map(section => ({
      title: section.props.title,
      rows: section.children.map(row => ({
        icon: row.props.icon, title: row.props.title,
        actions: row.props.actions.children.map(a => ({
          type: a.type, ...(a.type === 'Action.Paste' ? { content: a.props.content } : { url: a.props.url }),
        })),
      })),
    })),
  };
}
if (process.argv[2] === '--capture') console.log(JSON.stringify(captureOriginal(), null, 2));
