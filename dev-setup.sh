#!/usr/bin/env bash

pushd $(dirname $(realpath $0)) > /dev/null

# Copy git hook.
git config --unset core.hooksPath  # Ensure hooksPath is unset.
cp pre-commit .git/hooks/
chown $(logname) .git/hooks/pre-commit

echo ""
echo "Pre-commit hook installed!"
echo ""

popd > /dev/null
