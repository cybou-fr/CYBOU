#!/usr/bin/env bash
cd /mnt/c/Users/cybou/Documents/CYBOU
for g in vm-smoke m6-recovery-boundary; do
  if nix build --no-link ".#checks.x86_64-linux.$g" > /dev/null 2>&1; then
    echo "$g ok"
  else
    echo "$g FAILED"
  fi
done
