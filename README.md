# Ad Block engine in Rust

Native Rust module for Adblock Plus syntax (e.g. EasyList, EasyPrivacy) filter parsing and matching.

It uses a tokenisation approach for qucikly reducing the potentially matching rule search space against a URL.

The algorithm is inspired by, and closely follows the algorithm of [uBlock Origin](https://github.com/gorhill/uBlock) and [Cliqz](https://github.com/cliqz-oss/adblocker).

Somewhat graphical explanation of the algorithm:

![Ad Block Algorithm](./docs/algo.png "Ad Block Algorithm")

## Demo

Demo use in Rust:

```
extern crate adblock;

use adblock::engine::Engine;

#[test]
fn check_simple_use() {
    let rules = vec![
        String::from("-advertisement-icon."),
        String::from("-advertisement-management/"),
        String::from("-advertisement."),
        String::from("-advertisement/script."),
    ];

    let blocker = Engine::from_rules(&rules);
    let blocker_result = blocker.check_network_urls("http://example.com/-advertisement-icon.", "http://example.com/helloworld", "image");
    assert!(blocker_result.matched);
}

```


## TODO

- [ ] Generate redirect addresses based on provided resources.txt (uBo style)
- [ ] Function for extracting CSP directives
- [ ] Generate string representation of a rule when debug mode is off (i.e. initial rule is not available)
- [ ] Cosmetic filters
