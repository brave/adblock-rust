#!/usr/bin/env node
'use strict';

// Builds the native addon by invoking cargo and copying the cdylib to js/index.node.
// Replaces @napi-rs/cli to avoid pulling in unnecessary npm dependencies.

const { execSync } = require('child_process');
const { copyFileSync } = require('fs');
const { join } = require('path');

const release = process.argv.includes('--release');
const cargoArgs = ['build', '--manifest-path', 'js/Cargo.toml', '--message-format=json'];
if (release) cargoArgs.push('--release');

const output = execSync(`cargo ${cargoArgs.join(' ')}`, {
  stdio: ['inherit', 'pipe', 'inherit'],
  encoding: 'utf8',
});

// Find the cdylib artifact path from cargo's JSON output
let cdylibPath;
for (const line of output.split('\n')) {
  if (!line) continue;
  try {
    const msg = JSON.parse(line);
    if (msg.reason === 'compiler-artifact' && msg.target?.crate_types?.includes('cdylib')) {
      cdylibPath = msg.filenames[0];
    }
  } catch {}
}

if (!cdylibPath) {
  console.error('Error: could not find cdylib artifact in cargo output');
  process.exit(1);
}

const dest = join(__dirname, 'index.node');
copyFileSync(cdylibPath, dest);
console.log(`Copied ${cdylibPath} -> ${dest}`);
