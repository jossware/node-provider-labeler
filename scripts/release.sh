#!/bin/bash

# Exit immediately if a command exits with a non-zero status
set -e

increment_version() {
    local version=$1
    local major
    local minor
    local patch

    IFS='.' read -r major minor patch <<< "$version"
    minor=$((minor + 1))
    patch=0

    echo "$major.$minor.$patch"
}

extract_version_from_cargo_toml() {
    local file_path=$1
    grep -E '^version = "[0-9]+\.[0-9]+\.[0-9]+"' "$file_path" | cut -d '"' -f 2
}

extract_chart_version_from_chart_yaml() {
    local file_path=$1
    grep -E '^version: "[0-9]+\.[0-9]+\.[0-9]+"' "$file_path" | cut -d '"' -f 2
}

bump_version_in_cargo_toml() {
    local file_path=$1
    local new_version=$2
    sed -i.bak -E "s/^version = \".*\"/version = \"$new_version\"/" "$file_path"
    rm "${file_path}.bak"
}

bump_appversion_in_chart_yaml() {
    local file_path=$1
    local new_version=$2
    sed -i.bak -E "s/^appVersion: \".*\"/appVersion: \"$new_version\"/" "$file_path"
    rm "${file_path}.bak"
}

bump_appversion_in_values_yaml() {
    local file_path=$1
    local new_version=$2
    sed -i.bak -E 's/^([[:space:]]*tag:[[:space:]]*").*(")$/\1'"v$new_version"'\2/' "$file_path"
    rm "${file_path}.bak"
}

bump_chart_version_in_chart_yaml() {
    local file_path=$1
    local new_version=$2
    sed -i.bak -E "s/^version: \".*\"/version: \"$new_version\"/" "$file_path"
    rm "${file_path}.bak"
}

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <release_type>"
    echo "release_type: app | chart | both"
    exit 1
fi

release_type=$1

cargo_toml_path="Cargo.toml"
chart_yaml_path="charts/node-provider-labeler/Chart.yaml"
values_yaml_path="charts/node-provider-labeler/values.yaml"
kustomize_base_path="kustomize"

if [ "$release_type" == "app" ] || [ "$release_type" == "both" ]; then
    current_version=$(extract_version_from_cargo_toml "$cargo_toml_path")
    new_version=$(increment_version "$current_version")
    bump_version_in_cargo_toml "$cargo_toml_path" "$new_version"
    bump_appversion_in_chart_yaml "$chart_yaml_path" "$new_version"
    bump_appversion_in_values_yaml "$values_yaml_path" "$new_version"
    echo "Bumped application version to $new_version in Cargo.toml, appVersion in Chart.yaml, and image.tag in values.yaml"
fi

if [ "$release_type" == "chart" ] || [ "$release_type" == "both" ]; then
    current_chart_version=$(extract_chart_version_from_chart_yaml "$chart_yaml_path")
    new_chart_version=$(increment_version "$current_chart_version")
    bump_chart_version_in_chart_yaml "$chart_yaml_path" "$new_chart_version"
    echo "Bumped chart version to $new_chart_version in Chart.yaml"

    # Re-generate Helm Chart readme
    helm-docs --chart-search-root charts

    # Generate kustomize base
    konvert -f "$kustomize_base_path"

    # publish artifacthub metadata, must be authenticated to ghcr.io
    pushd charts/node-provider-labeler
    oras push \
        ghcr.io/jossware/charts/node-provider-labeler:artifacthub.io \
        --config /dev/null:application/vnd.cncf.artifacthub.config.v1+yaml \
        artifacthub-repo.yml:application/vnd.cncf.artifacthub.repository-metadata.layer.v1.yaml
    popd
fi

cargo build
