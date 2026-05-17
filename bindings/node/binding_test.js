/// <reference types='node' />

const assert = require('node:assert');
const {test} = require('node:test');

let TreeSitter;
try {
  TreeSitter = require('tree-sitter');
} catch (err) {
  if (err.code !== 'MODULE_NOT_FOUND') throw err;
}

test('can load grammar', {skip: TreeSitter ? false : 'tree-sitter peer not installed'}, () => {
  const parser = new TreeSitter();
  assert.doesNotThrow(() => parser.setLanguage(require('.')));
});
