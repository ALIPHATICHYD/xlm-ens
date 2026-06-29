# Contributing

We welcome contributions from the community! Please follow these guidelines to ensure a smooth development process.

## Commit Messages

All commit messages and Pull Request titles must follow the [Conventional Commits](https://www.conventionalcommits.org/) specification. This helps us automatically generate changelogs and release notes.

### Format

The commit message format is:

```
<type>(<scope>): <short description>
```

- **type**: `feat`, `fix`, `refactor`, `test`, `ci`, `docs`, `chore`, `perf`, `security`
- **scope**: `registry`, `registrar`, `resolver`, `auction`, `subdomain`, `nft`, `bridge`, `sdk`, `cli`, `common`

### Pre-commit Hook (Recommended)

To automatically validate your commit messages, you can use a pre-commit hook.

1.  **Install Husky and commitlint:**

    If you have Node.js and npm installed, you can use `husky` and `commitlint` to validate your commit messages before you commit.

    ```bash
    npm install --save-dev @commitlint/cli @commitlint/config-conventional husky
    npx husky install
    npx husky add .husky/commit-msg 'npx --no -- commitlint --edit "$1"'
    ```

    You will also need a `package.json` file in the root of the project. If you don't have one, you can create it by running `npm init -y`.

2.  **Use the `.commitlintrc.yml` configuration:**

    The `.commitlintrc.yml` file in the root of the repository contains the configuration for `commitlint`.

    ```yaml
    # .commitlintrc.yml
    rules:
      "type-enum":
        - 2
        - "always"
        - - feat
          - fix
          - refactor
          - test
          - ci
          - docs
          - chore
          - perf
          - security
      "scope-enum":
        - 2
        - "always"
        - - registry
          - registrar
          - resolver
          - auction
          - subdomain
          - nft
          - bridge
          - sdk
          - cli
          - common
    ```

## Snapshot Updates

If a test intentionally changes snapshot output, regenerate the snapshots,
review the diff, and commit the updated files together with the code change:

```sh
UPDATE_SNAPSHOTS=1 cargo test --test registrar_registry_test
git diff -- tests/test_snapshots
```

CI also checks `tests/test_snapshots/` after the test suite runs and fails if
new or modified snapshot files are left uncommitted.
