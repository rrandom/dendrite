#!/usr/bin/env node
/**
 * Sync version from .version file to Cargo.toml and package.json
 * Usage: node scripts/sync_version.js
 */

const fs = require('fs');
const path = require('path');

const projectRoot = path.join(__dirname, '..');
const versionFile = path.join(projectRoot, '.version');
const cargoToml = path.join(projectRoot, 'Cargo.toml');
const packageJson = path.join(projectRoot, 'clients', 'vscode', 'package.json');

// Read version from .version file
let version;
try {
  version = fs.readFileSync(versionFile, 'utf8').trim();
  if (!version) {
    console.error('Error: .version file is empty');
    process.exit(1);
  }
} catch (err) {
  console.error('Error: Could not read .version file:', err.message);
  process.exit(1);
}

console.log(`Found version: ${version}`);

// Update Cargo.toml
try {
  let cargoContent = fs.readFileSync(cargoToml, 'utf8');
  cargoContent = cargoContent.replace(/version\s*=\s*"[^"]+"/, `version = "${version}"`);
  fs.writeFileSync(cargoToml, cargoContent, 'utf8');
  console.log('Updated Cargo.toml');
} catch (err) {
  console.error('Error: Could not update Cargo.toml:', err.message);
  process.exit(1);
}

// Update package.json
try {
  const pkg = JSON.parse(fs.readFileSync(packageJson, 'utf8'));
  pkg.version = version;
  fs.writeFileSync(packageJson, JSON.stringify(pkg, null, 2) + '\n', 'utf8');
  console.log('Updated package.json');
} catch (err) {
  console.error('Error: Could not update package.json:', err.message);
  process.exit(1);
}

console.log(`✅ Version ${version} synced successfully`);

