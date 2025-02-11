window.BENCHMARK_DATA = {
  "lastUpdate": 1739243628404,
  "repoUrl": "https://github.com/brave/adblock-rust",
  "entries": {
    "Rust Benchmark": [
      {
        "commit": {
          "author": {
            "email": "matuchin@brave.com",
            "name": "Mikhail",
            "username": "atuchin-m"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7919bdd13e5dc9173c43d521331571392172598c",
          "message": "Merge pull request #418 from brave/setup-basic-perf-ci-follow-up\n\nFollow up for setup perf CI",
          "timestamp": "2025-01-29T00:13:35+04:00",
          "tree_id": "d2cb1c33575a3395abd7e7e0afe793fc18ec1f11",
          "url": "https://github.com/brave/adblock-rust/commit/7919bdd13e5dc9173c43d521331571392172598c"
        },
        "date": 1738095431371,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1745226241,
            "range": "± 10688991",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 1003256,
            "range": "± 7610",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 210108247,
            "range": "± 7007989",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-initial",
            "value": 41409969,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-after-1000-requests",
            "value": 44005995,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "73575789+boocmp@users.noreply.github.com",
            "name": "Pavel Beloborodov",
            "username": "boocmp"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "98aa69f7e317dda2c026c83d350dd5fc0cc64c56",
          "message": "Merge pull request #425 from brave/code_structure\n\nCode structure",
          "timestamp": "2025-02-11T10:10:11+07:00",
          "tree_id": "deaccedb6db790ebe80d1cb95a15d522400f253d",
          "url": "https://github.com/brave/adblock-rust/commit/98aa69f7e317dda2c026c83d350dd5fc0cc64c56"
        },
        "date": 1739243627118,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1717001323,
            "range": "± 16907239",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 989900,
            "range": "± 15044",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 202730589,
            "range": "± 4420888",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-initial",
            "value": 41409969,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-after-1000-requests",
            "value": 44005995,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}