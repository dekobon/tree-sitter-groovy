vim.filetype.add({
  extension = {
    gvy = "groovy",
    gy = "groovy",
    -- `*.jenkinsfile` is uncommon vs the canonical `Jenkinsfile`, but
    -- declared in tree-sitter.json's file-types and used by some teams
    -- to give editors a syntax-detection hook (see
    -- https://github.com/microsoft/vscode/issues/105325).
    jenkinsfile = "groovy",
  },
  pattern = {
    -- `Jenkinsfile.ci`, `Jenkinsfile.release`, etc. — common in repos
    -- with multiple pipelines. Neovim's built-in detection only
    -- matches the bare filename. Patterns are anchored to avoid
    -- false matches like `myJenkinsfile.bak`.
    [".*/Jenkinsfile%.[^/]+$"] = "groovy",
    ["^Jenkinsfile%.[^/]+$"] = "groovy",
  },
})
