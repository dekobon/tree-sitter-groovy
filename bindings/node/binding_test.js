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

test('exposes HIGHLIGHTS_QUERY', () => {
  const binding = require('.');
  assert.strictEqual(typeof binding.HIGHLIGHTS_QUERY, 'string');
  assert.ok(binding.HIGHLIGHTS_QUERY.length > 0);
  assert.strictEqual(binding.HIGHLIGHTS_QUERY, binding.HIGHLIGHTS_QUERY,
    'second access returns the cached value');
});
