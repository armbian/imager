# Contributing to Armbian Imager

Contributions are welcome. A few ways to help:

- Report bugs by [opening an issue](https://github.com/armbian/imager/issues)
- Suggest features, and open a discussion first if it's a big one
- Send pull requests for fixes and improvements
- Add or improve translations in `src/locales/`
- Improve the docs so the next person has an easier start

For environment setup, build instructions, and project structure, see [DEVELOPMENT.md](DEVELOPMENT.md).

## Development Workflow

1. Fork the repository.
2. Create a branch (`git checkout -b feature/amazing-feature`).
3. Make your changes and run the quality checks below.
4. Commit with a conventional commit message.
5. Push the branch (`git push origin feature/amazing-feature`).
6. Open a pull request.

### Branch naming

Prefix the branch with the kind of change: `feature/` for new functionality, `fix/` for bug fixes, `docs/` for documentation, `refactor/` for restructuring.

### Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>): <subject>`.

Types are `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `chore`, `build`, `ci`. Common scopes are `frontend`, `backend`, `flash`, `device`, `i18n`, `build`, `ci`, `docs`. Keep each commit to one logical change, and write the subject in the imperative ("add dark mode", not "added dark mode").

## Quality Checks

CI runs these on every pull request, so run them locally first.

Frontend, after touching anything under `src/`:

```bash
npm run lint        # ESLint
npx tsc --noEmit    # TypeScript type check
```

Backend, after touching anything under `src-tauri/`:

```bash
cd src-tauri
cargo fmt           # must produce no diff
cargo clippy --all-targets --all-features -- -D warnings
```

A PR needs zero lint errors and zero warnings to merge.

## Translations

The app ships 18 languages: `de`, `en`, `es`, `fr`, `hr`, `it`, `ja`, `ko`, `nl`, `pl`, `pt`, `pt-BR`, `ru`, `sl`, `sv`, `tr`, `uk`, `zh`. Note that `pt` and `pt-BR` are separate locales.

When you add or remove a translation key, change all 18 files in `src/locales/` so they keep the same set of keys. PRs that leave keys missing in some files won't pass review.

## Other ways to contribute

* [Become a new board maintainer](https://docs.armbian.com/Board_Maintainers_Procedures_and_Guidelines/)
* [Apply for an open position](https://forum.armbian.com/staffapplications/)
* [Help cover running costs](https://forum.armbian.com/subscriptions/)
* [Help community members in the forum](https://forum.armbian.com/)
* [Check the forum announcements for requests for help](https://forum.armbian.com/forum/37-announcements/)
