package services

// ExcludedFolders contains a comprehensive list of folders to exclude from git repository scanning.
// This includes build artifacts, dependencies, and cache directories for multiple programming languages.
var ExcludedFolders = map[string]bool{
	// ============================================================================
	// DEPENDENCIES AND PACKAGE MANAGERS
	// ============================================================================

	// JavaScript / TypeScript / Node.js
	"node_modules":       true,
	".npm":               true,
	".pnp":               true,
	".yarn":              true,

	// Python
	"venv":               true,
	".venv":              true,
	"env":                true,
	"ENV":                true,
	".Python":            true,
	"site-packages":      true,
	".eggs":              true,
	"dist-info":          true,

	// Go
	"vendor":             true,

	// Java / Kotlin
	".gradle":            true,
	".m2":                true,

	// Ruby
	".bundle":            true,
	".gems":              true,

	// C# / .NET
	"packages":           true,

	// Cocoapods (iOS)
	"Pods":               true,
	".pods":              true,

	// ============================================================================
	// BUILD OUTPUTS AND ARTIFACTS
	// ============================================================================

	// Generic
	"dist":               true,
	"build":              true,
	"out":                true,
	"bin":                true,
	"lib":                true,
	"target":             true,
	"obj":                true,

	// JavaScript / TypeScript frameworks
	".next":              true,
	".nuxt":              true,
	".webpack":           true,
	"out-tsc":            true,
	".angular":           true,

	// Python
	"__pycache__":        true,
	".pytest_cache":      true,
	".coverage":          true,
	"htmlcov":            true,
	".tox":               true,

	// C++ / CMake
	"cmake-build-debug":  true,
	"cmake-build-release": true,
	"_deps":              true,
	"_build":             true,

	// ============================================================================
	// IDE AND EDITOR CONFIGURATION
	// ============================================================================

	".vscode":            true,
	".idea":              true,
	".vs":                true,
	".vscode-test":       true,
	".sublime-text":      true,
	".eclipse":           true,
	".netbeans":          true,
	".project":           true,
	".classpath":         true,
	".c9":                true,
	".workspace":         true,

	// ============================================================================
	// VERSION CONTROL SYSTEMS
	// ============================================================================

	".git":               true,
	".svn":               true,
	".hg":                true,
	".bzr":               true,
	".pijul":             true,

	// ============================================================================
	// CONTINUOUS INTEGRATION / DEPLOYMENT
	// ============================================================================

	".github":            true,
	".gitlab":            true,
	".gitea":             true,
	".bitbucket":         true,
	".circleci":          true,
	".travis":            true,
	".jenkins":           true,
	"azure-pipelines":    true,
	".github-workflows":  true,

	// ============================================================================
	// TESTING AND COVERAGE
	// ============================================================================

	"coverage":           true,
	".nyc_output":        true,
	"test-results":       true,
	"lcov-report":        true,

	// ============================================================================
	// CACHE AND TEMPORARY FILES
	// ============================================================================

	".cache":             true,
	".tmp":               true,
	"tmp":                true,
	"temp":               true,
	".parcel-cache":      true,
	".eslintcache":       true,
	".jest-cache":        true,
	".node-gyp":          true,
	".npm-global":        true,

	// ============================================================================
	// OPERATING SYSTEM AND SYSTEM DIRECTORIES
	// ============================================================================

	".Spotlight-V100":    true,
	".Trashes":           true,
	"$RECYCLE.BIN":       true,

	// ============================================================================
	// OTHER COMMON NON-PROJECT DIRECTORIES
	// ============================================================================

	"site":               true,
	"docs-build":         true,
	".tsc-out":           true,
	"typings":            true,
}
