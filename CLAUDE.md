# Update docs

Constantly update these documents without repeating information between them. For changes, don't add them as additional notes of the change, just replace the old content with the updated one. Remove stale information.

- DESIGN.md with the functional design of the app.
- ./FRONTEND.md with the behavior of the frontend apps. Any change applied to the SwiftUI app (macos/) should be documented and also be applied to Tauri app (desktop/). FRONTEND.md stays as the source of truth of what the frontend apps do.
- TECHNICAL.md with the technical specifications of the app.
- README.md with a brief introduction to the app.
- ROADMAP.md with a concise checklist of things that had been done and are pending.

# Clean Code

Follow DRY and Single Responsibility principles

For Rust, always run clippy pedantic to see if our code follows official recommendations.

## Always fix

Do not ignore the errors or warnings when building or running the projects, even if they were already present or not caused by your changes. Do not bypass errors or warnings, actually fix them.