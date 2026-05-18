const path = require('path');
const root = path.join(__dirname, '..', '..');

module.exports = require('node-gyp-build')(root);

try {
  module.exports.nodeTypeInfo = require('../../src/node-types.json');
} catch {
  // ignore if file is missing
}

Object.defineProperty(module.exports, 'HIGHLIGHTS_QUERY', {
  configurable: true,
  enumerable: true,
  get() {
    delete module.exports.HIGHLIGHTS_QUERY;
    try {
      module.exports.HIGHLIGHTS_QUERY = require('fs').readFileSync(
        path.join(root, 'queries', 'groovy', 'highlights.scm'),
        'utf8',
      );
    } catch {
      // ignore if file is missing
    }
    return module.exports.HIGHLIGHTS_QUERY;
  },
});
