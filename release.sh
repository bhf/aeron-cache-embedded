#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e

if [ -z "$1" ]; then
  echo "Usage: ./release.sh <new_version>"
  echo "Example: ./release.sh 1.0.1"
  exit 1
fi

NEW_VERSION=$1
echo "🚀 Bumping versions across all libraries to $NEW_VERSION..."

echo "📦 Updating Java (build.gradle.kts)..."
sed -i -e "s/^version = \".*\"/version = \"$NEW_VERSION\"/" libraries/java/build.gradle.kts

echo "📦 Updating TypeScript (package.json)..."
(cd libraries/typescript && npm --no-git-tag-version version $NEW_VERSION)

echo "📦 Updating Python (pyproject.toml)..."
sed -i -e "s/^version = \".*\"/version = \"$NEW_VERSION\"/" libraries/python/pyproject.toml

echo "📦 Updating Rust (Cargo.toml)..."
# Replaces the first occurrence of version = "..." under [package]
sed -i -e "0,/^version = \".*\"/s/^version = \".*\"/version = \"$NEW_VERSION\"/" libraries/rust/Cargo.toml

echo "💾 Committing version bumps..."
git add libraries/java/build.gradle.kts libraries/typescript/package.json libraries/typescript/package-lock.json libraries/python/pyproject.toml libraries/rust/Cargo.toml
git commit -m "chore: bump version to $NEW_VERSION across all clients"

echo "🏷️  Tagging releases..."
git tag "java-v$NEW_VERSION"
git tag "ts-v$NEW_VERSION"
git tag "py-v$NEW_VERSION"
git tag "rust-v$NEW_VERSION"

echo "☁️  Pushing changes and tags to GitHub..."
git push origin HEAD
git push origin "java-v$NEW_VERSION" "ts-v$NEW_VERSION" "py-v$NEW_VERSION" "rust-v$NEW_VERSION"

echo ""
echo "✅ Successfully initiated release process for version $NEW_VERSION!"
echo "Check your GitHub Actions tab to monitor the publishing workflows."