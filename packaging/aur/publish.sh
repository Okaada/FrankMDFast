#!/usr/bin/env bash
# Push the frankmdfast-bin AUR package for a published GitHub release.
#
#   packaging/aur/publish.sh <version> <SHA256SUMS file>
#
# Runs as root in an archlinux container (CI). Needs AUR_SSH_PRIVATE_KEY,
# the private half of a key registered on the AUR account. The AUR host key
# is pinned (packaging/aur/known_hosts), never trusted on first use.
set -euo pipefail

version=$1 sums=$2
root=$(cd "$(dirname "$0")/../.." && pwd)
pkg=frankmdfast-bin

sha_for() { awk -v f="$1" '$2 == f || $2 == "*"f { print $1 }' "$sums"; }
sha_x86_64=$(sha_for "frankmdfast-$version-linux-x86_64.tar.gz")
sha_aarch64=$(sha_for "frankmdfast-$version-linux-aarch64.tar.gz")
if [ -z "$sha_x86_64" ] || [ -z "$sha_aarch64" ]; then
  echo "missing Linux tarball checksums in $sums" >&2
  exit 1
fi

ssh_dir=$(mktemp -d)
trap 'rm -rf "$ssh_dir"' EXIT
printf '%s\n' "$AUR_SSH_PRIVATE_KEY" > "$ssh_dir/key"
chmod 600 "$ssh_dir/key"
export GIT_SSH_COMMAND="ssh -i $ssh_dir/key -o IdentitiesOnly=yes -o StrictHostKeyChecking=yes -o UserKnownHostsFile=$root/packaging/aur/known_hosts"

work=$(mktemp -d)
chmod 755 "$work"
git clone "ssh://aur@aur.archlinux.org/$pkg.git" "$work/$pkg"
sed -e "s/@VERSION@/$version/" -e "s/@SHA256_X86_64@/$sha_x86_64/" -e "s/@SHA256_AARCH64@/$sha_aarch64/" \
  "$root/packaging/aur/$pkg/PKGBUILD" > "$work/$pkg/PKGBUILD"

# makepkg refuses to run as root. Check the published tarball matches, then
# generate .SRCINFO.
chown -R nobody "$work/$pkg"
(cd "$work/$pkg" && runuser -u nobody -- makepkg --verifysource --noconfirm)
(cd "$work/$pkg" && runuser -u nobody -- makepkg --printsrcinfo) > "$work/$pkg/.SRCINFO.new"
mv "$work/$pkg/.SRCINFO.new" "$work/$pkg/.SRCINFO"
rm -f "$work/$pkg"/*.tar.gz

cd "$work/$pkg"
git config --global --add safe.directory "$work/$pkg"
git config user.name "github-actions[bot]"
git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
git add PKGBUILD .SRCINFO
if git diff --cached --quiet; then
  echo "AUR package already at $version"
  exit 0
fi
git commit -m "Update to $version"
git push origin HEAD:master
