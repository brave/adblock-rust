window.BENCHMARK_DATA = {
  "lastUpdate": 1747643228996,
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
      },
      {
        "commit": {
          "author": {
            "email": "22821309+antonok-edm@users.noreply.github.com",
            "name": "Anton Lazarev",
            "username": "antonok-edm"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "1ac5df65a46b74c8e22d4430aa262f7924d39a90",
          "message": "Merge pull request #436 from brave/update-selectors\n\nBump selectors and cssparser (port to `master`)",
          "timestamp": "2025-02-21T17:28:30-08:00",
          "tree_id": "cfc29c32665c445fadbc713547ddc266067179fc",
          "url": "https://github.com/brave/adblock-rust/commit/1ac5df65a46b74c8e22d4430aa262f7924d39a90"
        },
        "date": 1740187920114,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1834765655,
            "range": "± 11475959",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 1015539,
            "range": "± 10628",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 217396665,
            "range": "± 4465397",
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
          "id": "5d024dc4af7d00f4cc52cda560f320be127832ff",
          "message": "Merge pull request #434 from brave/code_structure_4\n\nCode structure follow up #3.",
          "timestamp": "2025-02-27T08:56:45+07:00",
          "tree_id": "688f341753459ea1d815faa0865f467d664944f7",
          "url": "https://github.com/brave/adblock-rust/commit/5d024dc4af7d00f4cc52cda560f320be127832ff"
        },
        "date": 1740621610869,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1679767586,
            "range": "± 12080911",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 1007472,
            "range": "± 26154",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 214478745,
            "range": "± 3428695",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-initial",
            "value": 41408849,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-after-1000-requests",
            "value": 44004875,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "22821309+antonok-edm@users.noreply.github.com",
            "name": "Anton Lazarev",
            "username": "antonok-edm"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "af0c65a76d3f16c0071e610f423e42cf39a11eef",
          "message": "Merge pull request #442 from brave/bump-base64\n\nBump base64",
          "timestamp": "2025-03-13T14:02:06-07:00",
          "tree_id": "6d0865f4ae814cb980adcd8b2c826b9604616bce",
          "url": "https://github.com/brave/adblock-rust/commit/af0c65a76d3f16c0071e610f423e42cf39a11eef"
        },
        "date": 1741899932044,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1657920369,
            "range": "± 13523152",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 970929,
            "range": "± 12529",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 227719857,
            "range": "± 8671111",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-initial",
            "value": 41408849,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-after-1000-requests",
            "value": 44004875,
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
          "id": "b904779fff4f48122c0fde5cc12f1133c4e6b69e",
          "message": "Merge pull request #439 from brave/code_structure_5\n\nCode structure follow up #4\nThe implementation of NetworkFilterList has been moved to network_filter_list.rs.\nThe bitflags dependency version has been bumped to 2.9.0, seahash to 4.1.0.\nThe flatbuffers dependency has been added.\nFlatBuffers schema of the network filter list and the corresponding generated file have been added.",
          "timestamp": "2025-03-16T14:39:52+07:00",
          "tree_id": "dcd313484ef8e3ef42418aeccd3d2c87fbf6cd8e",
          "url": "https://github.com/brave/adblock-rust/commit/b904779fff4f48122c0fde5cc12f1133c4e6b69e"
        },
        "date": 1742110998439,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1700116474,
            "range": "± 16199179",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 1005898,
            "range": "± 5221",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 202512922,
            "range": "± 1989835",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-initial",
            "value": 41408849,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-after-1000-requests",
            "value": 44004875,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mplesa@brave.com",
            "name": "Mihai PLESA",
            "username": "mihaiplesa"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f7cd81cbb1e2810e7196774c71cdb78ab67688e5",
          "message": "Merge pull request #445 from brave/fix/autopin-deps-20250328002915011\n\nfix: autopin dependencies",
          "timestamp": "2025-03-28T10:08:44-04:00",
          "tree_id": "878f72405d10789703bfd42f6105caa6ef376f35",
          "url": "https://github.com/brave/adblock-rust/commit/f7cd81cbb1e2810e7196774c71cdb78ab67688e5"
        },
        "date": 1743171163908,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1724557739,
            "range": "± 23138921",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 1033498,
            "range": "± 13260",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 213162601,
            "range": "± 6261218",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-initial",
            "value": 41408849,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-after-1000-requests",
            "value": 44004875,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mplesa@brave.com",
            "name": "Mihai PLESA",
            "username": "mihaiplesa"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f89fcf1ac2b066b6991caa5c276e04a15b7773ce",
          "message": "Merge pull request #430 from brave/renovate/pin-dependencies\n\nchore(deps): pin dependencies",
          "timestamp": "2025-03-28T10:21:37-04:00",
          "tree_id": "35bcb32eed251361e9d805c4e5745954ebc4a3cb",
          "url": "https://github.com/brave/adblock-rust/commit/f89fcf1ac2b066b6991caa5c276e04a15b7773ce"
        },
        "date": 1743171912536,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1748060055,
            "range": "± 16444516",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 1003697,
            "range": "± 19747",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 220574560,
            "range": "± 4365524",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-initial",
            "value": 41408849,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-after-1000-requests",
            "value": 44004875,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "antonok35@gmail.com",
            "name": "Anton Lazarev",
            "username": "antonok-edm"
          },
          "committer": {
            "email": "antonok35@gmail.com",
            "name": "Anton Lazarev",
            "username": "antonok-edm"
          },
          "distinct": true,
          "id": "d56be21332b4ac28ce8e26b1273ac5cc979eb417",
          "message": "change npm lifecycle script from install to postinstall",
          "timestamp": "2025-04-17T16:03:15-07:00",
          "tree_id": "93fa944b84816d05b8f7f9e5f81f7a1f062744d4",
          "url": "https://github.com/brave/adblock-rust/commit/d56be21332b4ac28ce8e26b1273ac5cc979eb417"
        },
        "date": 1744932252683,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1727033355,
            "range": "± 11920489",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 1005963,
            "range": "± 12764",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 220140757,
            "range": "± 5159081",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-initial",
            "value": 41408849,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-after-1000-requests",
            "value": 44004875,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "antonok35@gmail.com",
            "name": "Anton Lazarev",
            "username": "antonok-edm"
          },
          "committer": {
            "email": "antonok35@gmail.com",
            "name": "Anton Lazarev",
            "username": "antonok-edm"
          },
          "distinct": true,
          "id": "67b7b70df169a7b669e028cb946d4159c39b5c25",
          "message": "support `$all`\n\nthis won't include `popup`, `inline-script`, or `inline-font` for now\ndue to lack of support for the individual options, but we may as well\nconvert `$all` into all supported types.",
          "timestamp": "2025-05-15T17:54:00-07:00",
          "tree_id": "775ef5fbd24fa569031b26b41fc853f241043642",
          "url": "https://github.com/brave/adblock-rust/commit/67b7b70df169a7b669e028cb946d4159c39b5c25"
        },
        "date": 1747357658794,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 1803081237,
            "range": "± 16195247",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 980016,
            "range": "± 5452",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 203961371,
            "range": "± 2004867",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-initial",
            "value": 41762172,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-after-1000-requests",
            "value": 44355700,
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
          "id": "50548c498d69a8c695e9eebb6f4f75eb7dc6eb80",
          "message": "Merge pull request #446 from brave/flatbuffers_impl\n\nFlatbuffers storage for internal filters representation.",
          "timestamp": "2025-05-19T15:23:19+07:00",
          "tree_id": "74e6c9692119644b029d54ad7d676a72998eb00d",
          "url": "https://github.com/brave/adblock-rust/commit/50548c498d69a8c695e9eebb6f4f75eb7dc6eb80"
        },
        "date": 1747643228159,
        "tool": "cargo",
        "benches": [
          {
            "name": "rule-match-browserlike/brave-list",
            "value": 2183228583,
            "range": "± 20870584",
            "unit": "ns/iter"
          },
          {
            "name": "rule-match-first-request/brave-list",
            "value": 1020696,
            "range": "± 15547",
            "unit": "ns/iter"
          },
          {
            "name": "blocker_new/brave-list",
            "value": 162880967,
            "range": "± 2078084",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-initial",
            "value": 21536659,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "memory-usage/brave-list-after-1000-requests",
            "value": 24141128,
            "range": "± 3",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}