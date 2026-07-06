# Update docs

Update (or create if they dont exist) the following docs. Always keep them concise and only keep latest information. Before adding a new section try updating an existing one.

- DESIGN.md with the functional design of the app.
- FRONTEND.md with the frontend behavior if any change is done to the app frontend.
- TECHNICAL.md with the technical specifications of the app.
- README.md with a brief introduction to the app.
- ROADMAP.md with a concise checklist of things that had been done and are pending.

# Clean Code

Follow DRY and Single Responsibility principles

For Rust, always run clippy pedantic to see if our code follows official recommendations.

## Always fix

Do not ignore the errors or warnings when building or running the projects, even if they were already present or not caused by your changes. Do not bypass errors or warnings, actually fix them.