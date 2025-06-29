import adblockRust from 'adblock-rs';
import fs from 'node:fs';
const dataPath = '../data/'

const debugInfo = true;
const filterSet = new adblockRust.FilterSet(debugInfo);

const easylistFilters = fs.readFileSync(
    dataPath + 'easylist.to/easylist/easylist.txt',
    { encoding: 'utf-8' },
).split('\n');
filterSet.addFilters(easylistFilters);

const uboUnbreakFilters = fs.readFileSync(
    dataPath + 'uBlockOrigin/unbreak.txt',
    { encoding: 'utf-8' },
).split('\n');
filterSet.addFilters(uboUnbreakFilters);

const resources = adblockRust.uBlockResources(
    dataPath + 'test/fake-uBO-files/web_accessible_resources',
    dataPath + 'test/fake-uBO-files/redirect-resources.js',
    dataPath + 'test/fake-uBO-files/scriptlets.js'
);

const engine = new adblockRust.Engine(filterSet, true);
engine.useResources(resources);

// Simple match
console.log(engine.check(
    'http://example.com/-advertisement-icon.',
    'http://example.com/helloworld',
    'image',
));
// Match with full details
console.log(engine.check(
    'http://example.com/-advertisement-icon.',
    'http://example.com/helloworld',
    'image',
    true,
));
// No match, but still with full details
console.log(engine.check(
    'https://github.githubassets.com/assets/frameworks-64831a3d.js',
    'https://github.com/brave',
    'script',
    true,
));
// Example that includes a redirect resource
console.log(engine.check(
    'https://bbci.co.uk/test/analytics.js',
    'https://bbc.co.uk',
    'script',
    true
));

// Serialize the engine to an ArrayBuffer
const serializedArrayBuffer = engine.serialize();
console.log(`Engine size: ${(serializedArrayBuffer.byteLength / 1024 / 1024).toFixed(2)} MB`);
