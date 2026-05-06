# Deskrona Context Enforcement

## MANDATORY: READ ON STARTUP

Whenever a session starts in the `Deskrona` (Deskrona) project, you MUST perform the following actions before any other work:

1. **Read `project.md`**: Use `view_file` to read the entire `project.md` file in the workspace root.
2. **Read `todo.md`**: Use `view_file` to read the entire `todo.md` file in the workspace root.

## Why This Skill Exists

This skill ensures that the agent always has the most up-to-date project context, preventing regression or loss of alignment with the roadmap.

## Project Paths

- Project Root: `./`
- Project Specs: `./project.md`
- Roadmap: `./todo.md`
