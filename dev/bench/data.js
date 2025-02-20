window.BENCHMARK_DATA = {
  "lastUpdate": 1740016625723,
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
      },
      {
        "commit": {
          "author": {
            "email": "shivankaulsahib@gmail.com",
            "name": "Shivan Kaul Sahib",
            "username": "ShivanKaul"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "c9a5b2f1fb744bde80c1ebc70a8d6ba16eed1f67",
          "message": "Merge pull request #429 from brave/no-panic-generic-procedural-filter\n\nSilently ignore generic procedural filter",
          "timestamp": "2025-02-12T12:07:33-08:00",
          "tree_id": "c01aeccee0168cf0162c66aac8ad1774ce86bc23",
          "url": "https://github.com/brave/adblock-rust/commit/c9a5b2f1fb744bde80c1ebc70a8d6ba16eed1f67"
        },
        "date": 1739391065834,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1793446288,
            "range": "± 21526190",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 989830,
            "range": "± 10171",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 205510823,
            "range": "± 4795405",
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
          "id": "4d3929839da7d69ff9cf3da2c15218da75866e3f",
          "message": "Merge pull request #428 from brave/code_structure_2\n\nCode structure follow up",
          "timestamp": "2025-02-13T17:19:54+07:00",
          "tree_id": "010001555cf2370b3f381ab9667560451544a790",
          "url": "https://github.com/brave/adblock-rust/commit/4d3929839da7d69ff9cf3da2c15218da75866e3f"
        },
        "date": 1739442214845,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1747067148,
            "range": "± 27669849",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 984611,
            "range": "± 8271",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 202003476,
            "range": "± 3378955",
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
          "id": "0fe35826be8a8149465a4c0ef97236fa52c9b66d",
          "message": "Merge pull request #433 from brave/code_structure_3\n\nCode structure follow up #2",
          "timestamp": "2025-02-20T08:53:25+07:00",
          "tree_id": "f676d84c674208939c0c7d9560edfb6cc9a20994",
          "url": "https://github.com/brave/adblock-rust/commit/0fe35826be8a8149465a4c0ef97236fa52c9b66d"
        },
        "date": 1740016625060,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1839697309,
            "range": "± 11629260",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 1015644,
            "range": "± 10433",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 209824147,
            "range": "± 3193249",
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