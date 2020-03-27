# Ad Block engine in Rust

[![Build Status](https://travis-ci.org/brave/adblock-rust.svg?branch=master)](https://travis-ci.org/brave/adblock-rust)

Native Rust module for Adblock Plus syntax (e.g. EasyList, EasyPrivacy) filter parsing and matching.

It uses a tokenisation approach for quickly reducing the potentially matching rule search space against a URL.

The algorithm is inspired by, and closely follows the algorithm of [uBlock Origin](https://github.com/gorhill/uBlock) and [Cliqz](https://github.com/cliqz-oss/adblocker).

Somewhat graphical explanation of the algorithm:

![Ad Block Algorithm](./docs/algo.png "Ad Block Algorithm")

## Demo

Demo use in Rust:

```rust
extern crate adblock;

use adblock::engine::Engine;

fn main() {
    let rules = vec![
        String::from("-advertisement-icon."),
        String::from("-advertisement-management/"),
        String::from("-advertisement."),
        String::from("-advertisement/script."),
    ];

    let blocker = Engine::from_rules_debug(&rules);
    let blocker_result = blocker.check_network_urls("http://example.com/-advertisement-icon.", "http://example.com/helloworld", "image");

    println!("Blocker result: {:?}", blocker_result);
}

```

## Node.js module demo

Note the Node.js module has overheads inherent to boundary crossing between JS and native code.

```js
const AdBlockClient = require('adblock-rs');
let el_rules = fs.readFileSync('./data/easylist.to/easylist/easylist.txt', { encoding: 'utf-8' }).split('\n');
let ubo_unbreak_rules = fs.readFileSync('./data/uBlockOrigin/unbreak.txt', { encoding: 'utf-8' }).split('\n');
let rules = el_rules.concat(ubo_unbreak_rules);
let resources = AdBlockClient.uBlockResources('uBlockOrigin/src/web_accessible_resources', 'uBlockOrigin/src/js/redirect-engine.js', 'uBlockOrigin/assets/resources/scriptlets.js');

// create client with debug = true
const client = new AdBlockClient.Engine(rules, true);
client.updateResources(resources);

const serializedArrayBuffer = client.serialize(); // Serialize the engine to an ArrayBuffer

console.log(`Engine size: ${(serializedArrayBuffer.byteLength / 1024 / 1024).toFixed(2)} MB`);

console.log("Matching:", client.check("http://example.com/-advertisement-icon.", "http://example.com/helloworld", "image"))
// Match with full debuging info
console.log("Matching:", client.check("http://example.com/-advertisement-icon.", "http://example.com/helloworld", "image", true))
// No, but still with debugging info
console.log("Matching:", client.check("https://github.githubassets.com/assets/frameworks-64831a3d.js", "https://github.com/AndriusA", "script", true))
// Example that inlcludes a redirect response
console.log("Matching:", client.check("https://bbci.co.uk/test/analytics.js", "https://bbc.co.uk", "script", true))
```


## TODO

- [ ] Function for extracting CSP directives
- [ ] Generate string representation of a rule when debug mode is off (i.e. initial rule is not available)
- [x] Cosmetic filters
