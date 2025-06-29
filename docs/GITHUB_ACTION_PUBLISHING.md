# Publishing Taskmaster-Sync as a GitHub Action

## Overview

There are three main approaches to publish taskmaster-sync as a reusable GitHub Action:

## 1. Composite Action (Recommended)

This approach downloads pre-built binaries from releases and runs them directly.

### action.yml in Repository Root

```yaml
name: 'Taskmaster Sync'
description: 'Synchronize TaskMaster tasks with GitHub Projects'
author: '5dlabs'

inputs:
  config-path:
    description: 'Path to sync configuration file'
    required: false
    default: '.taskmaster/sync-config.json'

  tag:
    description: 'TaskMaster tag to sync'
    required: false
    default: 'master'

  project-number:
    description: 'GitHub Project number (auto-detected if not specified)'
    required: false

  direction:
    description: 'Sync direction: to-github, from-github, or bidirectional'
    required: false
    default: 'to-github'

  github-token:
    description: 'GitHub token for authentication'
    required: false
    default: ${{ github.token }}

  version:
    description: 'Specific version of taskmaster-sync to use'
    required: false
    default: 'latest'

outputs:
  created:
    description: 'Number of items created'
    value: ${{ steps.sync.outputs.created }}

  updated:
    description: 'Number of items updated'
    value: ${{ steps.sync.outputs.updated }}

  deleted:
    description: 'Number of items deleted'
    value: ${{ steps.sync.outputs.deleted }}

runs:
  using: 'composite'
  steps:
    - name: Download taskmaster-sync
      shell: bash
      run: |
        # Determine version
        VERSION="${{ inputs.version }}"
        if [ "$VERSION" = "latest" ]; then
          VERSION=$(curl -s https://api.github.com/repos/${{ github.repository }}/releases/latest | jq -r .tag_name)
        fi

        # Determine platform
        case "${{ runner.os }}" in
          Linux)   PLATFORM="linux-x64" ;;
          macOS)   PLATFORM="darwin-x64" ;;
          Windows) PLATFORM="windows-x64.exe" ;;
        esac

        # Download binary
        curl -L -o taskmaster-sync \
          "https://github.com/${{ github.repository }}/releases/download/${VERSION}/taskmaster-sync-${PLATFORM}"
        chmod +x taskmaster-sync

    - name: Run sync
      id: sync
      shell: bash
      env:
        GITHUB_TOKEN: ${{ inputs.github-token }}
      run: |
        # Build command
        CMD="./taskmaster-sync sync"
        [ -n "${{ inputs.config-path }}" ] && CMD="$CMD --config ${{ inputs.config-path }}"
        [ -n "${{ inputs.tag }}" ] && CMD="$CMD --tag ${{ inputs.tag }}"
        [ -n "${{ inputs.project-number }}" ] && CMD="$CMD --project ${{ inputs.project-number }}"
        CMD="$CMD --direction ${{ inputs.direction }}"

        # Run and capture output
        OUTPUT=$($CMD --json)

        # Extract stats
        echo "created=$(echo $OUTPUT | jq -r .stats.created)" >> $GITHUB_OUTPUT
        echo "updated=$(echo $OUTPUT | jq -r .stats.updated)" >> $GITHUB_OUTPUT
        echo "deleted=$(echo $OUTPUT | jq -r .stats.deleted)" >> $GITHUB_OUTPUT

branding:
  icon: 'refresh-cw'
  color: 'blue'
```

## 2. Usage in Other Projects

Once published, other projects can use it like this:

```yaml
name: Sync Tasks

on:
  pull_request:
    types: [opened, synchronize]
  push:
    branches: [main]

jobs:
  sync:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Sync Tasks to GitHub Project
        uses: 5dlabs/taskmaster-sync@v1
        with:
          tag: ${{ github.event.pull_request.head.ref || 'main' }}
          direction: to-github
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

## 3. Publishing Process

### Option A: Direct Repository Publishing

1. Create a public repository (or use existing)
2. Add `action.yml` to the root
3. Create a release with binaries
4. Tag it (e.g., v1.0.0)
5. The action is immediately available as `owner/repo@v1`

### Option B: GitHub Marketplace Publishing

1. Complete Option A steps
2. Click "Draft a release" on GitHub
3. Check "Publish this Action to the GitHub Marketplace"
4. Fill in categories and description
5. Publish release

### Option C: Automated Publishing Workflow

Create `.github/workflows/publish-action.yml`:

```yaml
name: Publish Action

on:
  release:
    types: [published]

jobs:
  update-major-version:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Update major version tag
        run: |
          VERSION=${{ github.event.release.tag_name }}
          MAJOR_VERSION=$(echo $VERSION | cut -d. -f1)

          git config user.name github-actions
          git config user.email github-actions@github.com

          # Force update major version tag
          git tag -fa $MAJOR_VERSION -m "Update $MAJOR_VERSION tag"
          git push origin $MAJOR_VERSION --force
```

## 4. Alternative: Reusable Workflow

Instead of an action, you can publish a reusable workflow:

`.github/workflows/taskmaster-sync-reusable.yml`:

```yaml
name: Reusable Taskmaster Sync

on:
  workflow_call:
    inputs:
      tag:
        required: false
        type: string
        default: 'master'
      direction:
        required: false
        type: string
        default: 'to-github'
    secrets:
      github-token:
        required: false

jobs:
  sync:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Download and run taskmaster-sync
        env:
          GITHUB_TOKEN: ${{ secrets.github-token || github.token }}
        run: |
          # Download latest binary
          curl -L -o taskmaster-sync \
            https://github.com/5dlabs/taskmaster-sync/releases/latest/download/taskmaster-sync-linux-x64
          chmod +x taskmaster-sync

          # Run sync
          ./taskmaster-sync sync \
            --tag "${{ inputs.tag }}" \
            --direction "${{ inputs.direction }}"
```

Usage in other repos:

```yaml
jobs:
  sync:
    uses: 5dlabs/taskmaster-sync/.github/workflows/taskmaster-sync-reusable.yml@main
    with:
      tag: ${{ github.head_ref }}
      direction: to-github
```

## Benefits of GitHub Action Publishing

1. **Easy Integration**: Other projects just reference the action
2. **Version Control**: Users can pin to specific versions
3. **Marketplace Visibility**: Discoverable by other developers
4. **Automatic Updates**: Major version tags (v1, v2) can auto-update
5. **No Binary Management**: Action handles downloading correct binary

## Recommended Approach

1. Use the Composite Action approach (Option 1)
2. Automate binary building in your release workflow
3. Publish to GitHub Marketplace for visibility
4. Use semantic versioning with major version tags
5. Provide clear documentation and examples

This allows any project to add taskmaster-sync with just a few lines of YAML!