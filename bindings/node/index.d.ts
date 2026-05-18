type BaseNode = {
  type: string;
  named: boolean;
};

type ChildNode = {
  multiple: boolean;
  required: boolean;
  types: BaseNode[];
};

type NodeInfo =
  | (BaseNode & {
      subtypes: BaseNode[];
    })
  | (BaseNode & {
      fields: { [name: string]: ChildNode };
      children: ChildNode[];
    });

/**
 * The tree-sitter binding for this grammar.
 *
 * @see {@linkcode https://tree-sitter.github.io/node-tree-sitter/interfaces/Language.html Parser.Language}
 *
 * @example
 * const Parser = require("tree-sitter");
 * const Groovy = require("tree-sitter-groovy");
 *
 * const parser = new Parser();
 * parser.setLanguage(Groovy);
 */
declare const binding: {
  /**
   * The inner language object.
   * @private
   */
  language: unknown;

  /**
   * The content of the `node-types.json` file for this grammar.
   * Loaded eagerly on require; `undefined` if the file is missing.
   *
   * @see {@linkplain https://tree-sitter.github.io/tree-sitter/using-parsers/6-static-node-types Static Node Types}
   */
  nodeTypeInfo?: NodeInfo[];

  /**
   * The syntax highlighting query for this grammar
   * (`queries/groovy/highlights.scm`). Loaded lazily on first access;
   * `undefined` if the file is missing.
   */
  HIGHLIGHTS_QUERY?: string;
};

export = binding;
