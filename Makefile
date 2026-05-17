# Help
## ==============================
TREE_SITTER := ./node_modules/.bin/tree-sitter

help: ## Show this help
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z0-9_-]+:.*?## / {sub("\\\\n",sprintf("\n%22c"," "), $$2);printf "\033[36m%-25s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: build
build: parser/groovy.so ## Build the tree-sitter-groovy parser

parser/groovy.so: src/parser.c src/scanner.c | deps ## Compile parser C files into shared object
	$(RM) $@
	mkdir -p parser
	$(TREE_SITTER) build -o $@

src/parser.c: grammar.js | deps ## Generate parser source from grammar.js
	$(TREE_SITTER) generate

.PHONY: test
test: parser/groovy.so ## Run tree-sitter tests
	$(TREE_SITTER) test

.PHONY: clean
clean: ## Clean local environment
	rm -rf build node_modules parser

node_modules: package.json package-lock.json
	npm install
	touch node_modules

.PHONY: deps
deps: node_modules ## Install npm dependencies if needed

.PHONY: version
version: deps ## Tag new tree-sitter-groovy semver
	read -p "version: " version && \
	$(TREE_SITTER) version $$version

## Linting
.PHONY: lint
lint: deps ## Run eslint
	npm run lint
