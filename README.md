# Ad Block engine in Rust

Native Rust module for Adblock Plus (e.g. EasyList, EasyPrivacy) filter parsing and matching.

It uses a tokenisation approach for qucikly reducing the potentially matching rule search space against a URL.

The algorithm is inspired by, and closely follows the algorithm of [Cliqz](https://github.com/cliqz-oss/adblocker).

Somewhat graphical explanation of the algorithm:

![Ad Block Algorithm](./docs/algo.png "Ad Block Algorithm")

## Demo

Demo use in Rust:

```
extern crate adblock;

use adblock::lists::parse_filters;
use adblock::blocker::{Blocker, BlockerOptions};
use adblock::request::Request;

let rules = vec![
    String::from("-advertisement-icon."),
    String::from("-advertisement-management/"),
    String::from("-advertisement."),
    String::from("-advertisement/script."),
];
let (network_filters, _) = parse_filters(&rules, true, false, false);

let blocker_options = BlockerOptions {
    debug: false,
    enable_optimizations: false,
    load_cosmetic_filters: false,
    load_network_filters: true
};

let blocker = Blocker::new(network_filters, &blocker_options);

let maybeReq = Request::from_urls("http://example.com/-advertisement-icon.", "http://example.com/helloworld", "image");

assert!(maybeReq.is_ok(), "Request failed to parse");
let req = maybeReq.unwrap();
let blocker_result = blocker.check(&req);
assert!(blocker_result.matched);

```


## TODO

- [ ] Serialization and deserialization of fully initialised engine
- [ ] Rule optimisations (combining similar rules)
- [ ] Generate redirect addresses based on provided resources.txt (uBo style)
- [ ] Function for extracting CSP directives
- [ ] Generate string representation of a rule when debug mode is off (i.e. initial rule is not available)
- [ ] Cosmetic filters


