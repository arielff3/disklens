## What and why

<!-- What does this change, and what problem does it solve? -->

## Checklist

- [ ] `npm test` and `npm run build` pass
- [ ] `cargo test` and `cargo clippy --all-targets -- -D warnings` pass (in `src-tauri`)
- [ ] New UI text is in `src/strings.ts`; colors come from the themes
- [ ] Anything that deletes files only removes offered paths, passes `cleanup::safety::check`, and never follows junctions
- [ ] Screenshots for visible changes
