use once_cell::sync::Lazy;
use std::collections::HashSet;

pub static EXCLUDED_DIRS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();

    // Dependencies
    set.insert("node_modules");
    set.insert("vendor");
    set.insert("venv");
    set.insert(".venv");
    set.insert("env");
    set.insert(".env");
    set.insert("virtualenv");
    set.insert(".virtualenv");
    set.insert("Pods");
    set.insert("packages");
    set.insert(".gradle");
    set.insert(".dart_tool");
    set.insert(".flutter");
    set.insert("bower_components");
    set.insert("jspm_packages");

    // Build outputs
    set.insert("dist");
    set.insert("build");
    set.insert("out");
    set.insert("output");
    set.insert("bin");
    set.insert("obj");
    set.insert("target");
    set.insert("_build");
    set.insert("_site");
    set.insert("public");
    set.insert(".next");
    set.insert(".nuxt");
    set.insert(".output");
    set.insert(".vercel");
    set.insert(".netlify");

    // IDE/Editor
    set.insert(".idea");
    set.insert(".vscode");
    set.insert(".vs");
    set.insert(".fleet");
    set.insert(".eclipse");
    set.insert(".netbeans");

    // Cache
    set.insert(".cache");
    set.insert(".parcel-cache");
    set.insert(".turbo");
    set.insert(".nx");
    set.insert(".eslintcache");
    set.insert(".stylelintcache");
    set.insert("__pycache__");
    set.insert(".pytest_cache");
    set.insert(".mypy_cache");
    set.insert(".ruff_cache");

    // Version control
    set.insert(".git");
    set.insert(".svn");
    set.insert(".hg");
    set.insert(".bzr");

    // CI/CD
    set.insert(".github");
    set.insert(".gitlab");
    set.insert(".circleci");
    set.insert(".jenkins");

    // OS
    set.insert(".DS_Store");
    set.insert("Thumbs.db");
    set.insert("$RECYCLE.BIN");

    set
});
