use crate::lists::FilterList;

pub fn regions() -> Vec<FilterList> {
    [
        FilterList {
            uuid: String::from("9FCEECEC-52B4-4487-8E57-8781E82C91D0"),
            url: String::from("https://easylist-downloads.adblockplus.org/Liste_AR.txt"),
            title: String::from("Liste AR"),
            langs: [String::from("ar")].to_vec(),
            support_url: String::from("https://forums.lanik.us/viewforum.php?f=98"),
            component_id: String::from("gpgegghiabhggiplapgdfnfcmodkccji"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAnbHn298ZQjKnWlC6NgkvS3Dr7Neu87d1h8s3b9GTlc1QNDWiYgY5IfWVq/1FBw2nUFE/v8fNJg8quq8Z2nS8dYiJDVSGRggiCooa0OTCARL0BsGxHZO6s2QROYIcxPVnzISqg5zRIBc+8npE68uVUrDR6q/KdJ8siL2hrR/NybPp+uTK44lHOEIBFm8ih1rC6z+Y5dHfhax0CuL6wlWwVNcFe1macYEcOXShwkUOADh6rEBQZKJmv474xJutmB8nIpGq7C2Hn2HNNyfA6tYmhVlsaeEC44phGITKDai03wFsWWkHQPEU5HwFzKQGIBFwudyO8iigO5m+d3XSzgSZtQIDAQAB"),
            desc: String::from("Removes advertisements on Arabic websites")
        },
        FilterList {
            uuid: String::from("FD176DD1-F9A0-4469-B43E-B1764893DD5C"),
            url: String::from("https://stanev.org/abp/adblock_bg.txt"),
            title: String::from("Bulgarian Adblock list"),
            langs: [String::from("bg")].to_vec(),
            support_url: String::from("https://stanev.org/abp/"),
            component_id: String::from("coofeapfgmpkchclgdphgpmfhmnplbpn"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAoqpe6QWKketr66AW+hKSxr5qds9gxmU72N20bG/nG8wfbPUdTwEqIG3m5e37Iq1FI0gcQir5UqwZZUgkum5dWJwgCB1SOaVvlZrWZbTbATKFcePswHqAIXMfS+wzMA/Ifoky26dE4k/rs6J/zQEbeXXon/ikjGJ7GxYeHYMBz9DAPQhcUoBlh1C0O0vhvXU+kG5DO4wdIt9W/cXJtv+8OTZ6HiTJw1j0jAliFZI/jhkYB6MW57OBpBYlWJQhMbLbK5opXq6d4ELbjC1amqI1lT3j5bl0g1OpMqL4Jtz6578G79gMJfxE3hA5tL0rGU3vAmwck/jXh7uOOzqetwdBcwIDAQAB"),
            desc: String::from("Removes advertisements on Bulgarian websites")
        },
        FilterList {
            uuid: String::from("11F62B02-9D1F-4263-A7F8-77D2B55D4594"),
            url: String::from("https://easylist-downloads.adblockplus.org/easylistchina.txt"),
            title: String::from("EasyList China (中文)"),
            langs: [String::from("zh")].to_vec(),
            support_url: String::from("http://abpchina.org/forum/forum.php"),
            component_id: String::from("llhecljkijgcaalnbfadljdpkpbehakp"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAyahWbgHuWAI7CkBdclxOlehPVVGuG8u6bPi1vs016Kbhn9GThEVIP5qzAFQLA3jRrGy5B2nncdaCnibf7BkGsNR7nyQQuXAI2FGk9qCm36ZF7FI/yjtN0S0e6LzSswOcVhTdPnVkxYY6UDuKyzVRxgbF9Yg1aT45NFpJFZKFtKHexnLiY6KlZKV6GhY1jucjo7W77xdpLaspkYbQ69UvDlSA093InAzzikuqBdKvY0FPvC6pgiefqWTMa4M1cZU9IoIiukqrpXQn1tC9PJ8CU4XKCTshaNbpX5wxY10rUl7i/WHNcXCfmCXxKbqRZ1SyH6KiiBrDpSnfKXxrQip4GwIDAQAB"),
            desc: String::from("Removes advertisements on Chinese websites")
        },
        FilterList {
            uuid: String::from("CC98E4BA-9257-4386-A1BC-1BBF6980324F"),
            url: String::from("https://raw.githubusercontent.com/cjx82630/cjxlist/master/cjx-annoyance.txt"),
            title: String::from("CJX's Annoyance List"),
            langs: Vec::new(),
            support_url: String::from("https://github.com/cjx82630/cjxlist"),
            component_id: String::from("llpoppgpcimnmhgehpipdmamalmpfbjd"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAudVtKg3tZbgealGvVzbEL3yP1YWdt6GRDreqy/b3kCce3AZ8WroL8jb6Zj/aapBRCNxBezXzij+b6QiIH/l7sn5Wf5HDs5Vnrx4fDvGRtSLpgP0cSuFGVDx71TQz4X+AnUubOeHskIlJJAT4t4cHWs9c7EAl3ShG7DtvL2qHG2TUfJFqYOMOtQd2qG5H+X9zAUFP/qRHT55gzce8h+SXCsvdK4B8XK1cdvbIykllbGPzZr/TANn9gCtMKxUfk1qFn1uYD6mzg80KJmof8MHbLon6KLMqywcqfwEwvoivxo6f5LkOUjhqDYZEQ5la3h7lFfHKz7fCE7FCww7bQ028lwIDAQAB"),
            desc: String::from("Removes additional advertisements on Chinese websites, may break some websites")
        },
        FilterList {
            uuid: String::from("92AA0D3B-34AC-4657-9A5C-DBAD339AF8E2"),
            url: String::from("https://raw.githubusercontent.com/cjx82630/cjxlist/master/cjxlist.txt"),
            title: String::from("CJX's EasyList Lite (main focus on Chinese sites)"),
            langs: Vec::new(),
            support_url: String::from("https://github.com/cjx82630/cjxlist"),
            component_id: String::from("lgfeompbgommiobcenmodekodmdajcal"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAtAMyjMBZCbqNIuO01ZFJ5iKmcNFuJHXIUqhO9s6j6XnBAfak/OOk4s9k3maXyhaynXVpYATQyRHR0OEpmsQawFgKCmVm1LB68jxJ5Hh1ZITG1UyfznYnozkjBtzdkMGKeuZFBaHo5PPueHVO7yJDHvU3UFW4vCJ01twXiH4y0qaYjL1CPr58J9U0oKxptsfwEC53WcDq6mKtAKRpyxN6vbtFJ5/li2yC0Ms+8Xe3Xv5ovniM/4vNf3Jn1w0jzgrDRcW2VhxpydsH6q7oaR2igIzJ+XG6/k0g29CJhfT85dJNF31TwqvoI+Ju6hjZrEmSHmC7gbY7gN3ak+DbUrQxjwIDAQAB"),
            desc: String::from("Removes additional advertisements on Chinese websites, may break some websites")
        },
        FilterList {
            uuid: String::from("7CCB6921-7FDA-4A9B-B70A-12DD0A8F08EA"),
            url: String::from("https://raw.githubusercontent.com/tomasko126/easylistczechandslovak/master/filters.txt"),
            title: String::from("CZE, SVK: EasyList Czech and Slovak"),
            langs: [String::from("cs"), String::from("sk")].to_vec(),
            support_url: String::from("https://github.com/tomasko126/easylistczechandslovak"),
            component_id: String::from("omkkefoeihpbpebhhbhmjekpnegokpbj"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAuYBXbfloR5HddFlg80U8+pf5TqfFJQAf1bL4myp9KfGggwqrjuRzIkPOD8J8IvCp8tWv2f4QK9sAPHhtV6w1cnYX24lKxrQ/lHHAV6/CEcFa+2Yk7cRLKDC10H3r4FMRoCeAy/ruTjVPfIw+GuAfFYl1qYWBNxvW7XXw7cCIIYL4j82YQF6HjsWbTT+QHLCR6h66wvIyVQC9ppjJPxDaEevjt4tohEFAB1NBC+Wxt8H/P5r5ayNcLnb9Ygt75haYL8VWZOJhO/neSTyuidTFG5ox2Ruc6TXP8t0IqpVtiZUDkx1jzUakIHoKNMBc7oz3P/SQ4AanZsIliJobXFeUiQIDAQAB"),
            desc: String::from("Removes advertisements from Czech and Slovak websites")
        },
        FilterList {
            uuid: String::from("E71426E7-E898-401C-A195-177945415F38"),
            url: String::from("https://easylist-downloads.adblockplus.org/easylistgermany.txt"),
            title: String::from("EasyList Germany"),
            langs: [String::from("de")].to_vec(),
            support_url: String::from("https://forums.lanik.us/viewforum.php?f=90"),
            component_id: String::from("faknfgalcghekhfggcdikddilkpjbonh"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAu1CpR7Asj+2wl/1vM39WGUrHQ6vT+nuo+XSL7VzTaxW7g7el5lUC2X9MaaynfK7gOblr5Wnf/mjSJJZA57mxogjOCPP8lF0c7sOEgeO5L6hnDGB7sonCEFpHnBEn8VOZDvmqEb++AiXUPBFSnAOt4Mouck5CY80N6Sqbt4cxUBSof/NsGHZiTvCN7fJpW4ajLOtbWhCAmBhdG0VHatBG+Et/Z6yQtxEYQixKQNHJljiq55MzuE2jfGOZ8MAjyQdstF+GGfF6WPqnR5fd1rECK3OsI8zV9OOLPkjKrKEnlMsaMFFFU0T7Ly1UALehlWXtunelzq1mGvVS7vV+5aVR/QIDAQAB"),
            desc: String::from("Removes advertisements from German websites")
        },
        FilterList {
            uuid: String::from("9EF6A21C-5014-4199-95A2-A82491274203"),
            url: String::from("https://adblock.dk/block.csv"),
            title: String::from("Schacks Adblock Plus liste"),
            langs: [String::from("da")].to_vec(),
            support_url: String::from("https://henrik.schack.dk/adblock/"),
            component_id: String::from("facajiciiepdpjnoifonbfgcnlbpbieo"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAxvmMjXthrTjGl0WDrL+4qYSVao5vKPGerr2VeUiZ1o94+P0IJyZzq3d7UP2nOvhveGl15YYxYss+sD/sUkUW57XMx+H4TF5OzGCwV8nkz4VoMIfEU6CKgYmRGHV2VoMdIHG7R++jX20+GAoeBw+aBx9+AHlBouf1kvqbkutVh+Bre1cVa6YsgsPVcmhiEp7wjz2yB23f44+pBIQgWlKWn7z9e1osG4LUCGk6gavtRoNGS3TAUf1Sq9EUibFJVmBjujVoiQKD8GIFKmLM9Fxl1Q+xgG2PCCSBz5lSesHkphDpwhszedurpKbWsnsRPqbqR3GmpceKQheWL/Y56tf2gwIDAQAB"),
            desc: String::from("Removes advertisements from Danish websites")
        },
        FilterList {
            uuid: String::from("0783DBFD-B5E0-4982-9B4A-711BDDB925B7"),
            url: String::from("https://adblock.ee/list.php"),
            title: String::from("Eesti saitidele kohandatud filter"),
            langs: [String::from("et")].to_vec(),
            support_url: String::from("https://adblock.ee/"),
            component_id: String::from("fnpjliiiicbbpkfihnggnmobcpppjhlj"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAnrl1tavPfozqu7CmqNfVUtZfUlIitbWpFBRn+HVW0oEFUNqAwNlwHqy9QZP88wKvb5N3EJj6NAq6je4ii6nMkDn59teNzGA4m8QSkeOWT6pNm98FZA6HNHPnhnYSG2sT8tpQ8Uyh4ySrxj2ijVM0Hc01WKQ6zjkvZWOuZWllsCejRZmxGOLUUy5mtKhIfHiuleZ7AmKx46AiVFvrpvV5x8G2HKAlF/uDc6LmV0lfXcROt5RlY+kD/sQ6wKcatibpHbLoRHOJx3ac13+pvt85773af0MdrvdCYjxvqn3DJlKw9qqk/B59n+XdTmWcfC9k77Z0teoMM5EBy8G1nGbelwIDAQAB"),
            desc: String::from("Removes advertisements from Estonian websites")
        },
        FilterList {
            uuid: String::from("AC023D22-AE88-4060-A978-4FEEEC4221693"),
            url: String::from("https://easylist-downloads.adblockplus.org/easylist-cookie.txt"),
            title: String::from("Easylist-Cookie List - Filter Obtrusive Cookie Notices"),
            langs: Vec::new(),
            support_url: String::from("https://forums.lanik.us/"),
            component_id: String::from("lfgnenkkneohplacnfabidofpgcdpofm"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqNcRp37CBZCXe1vbmXyobycPxHyEKNIgNl6p0XBBxtcZcQOijpY70GjCRzgCL7m1+FBo4MR3FXLiF2aPn/QsUR8t7+zfw3XzBVos4Ssexkqpd4/4ciASwTXpbuyFOq4Z5dcgJ1afeT9Zj5bmh4ekLpgJ1NzVwCMhEKk6cmSKIaGVo5EEydtlor2nkUJrSFuZA6tYZ++4BOfhhCtzrvXTZjg7mTlB6ca21NL4oLwtqvJMtF8ddoumh619BB5wOqxLzntC/oWyOxf00V5HDC7e/DRj9J8jLRFLd4EQUO4Mk+kG3MNy0ph9cqdw6zFR7a2H3LGkl4ejsifM1mUDuJL0cwIDAQAB"),
            desc: String::from("Removes obtrusive cookie law notices")
        },
        FilterList {
            uuid: String::from("1C6D8556-3400-4358-B9AD-72689D7B2C46"),
            url: String::from("https://raw.githubusercontent.com/finnish-easylist-addition/finnish-easylist-addition/master/Finland_adb.txt"),
            title: String::from("Finnish Addition to Easylist"),
            langs: [String::from("fi")].to_vec(),
            support_url: String::from("https://github.com/finnish-easylist-addition/finnish-easylist-addition"),
            component_id: String::from("kdcalgmhljnckmnfcboeabeepgnlaemf"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA3seBXoyYSdtiqNAIaS5v9jP6Pr8xqgFnZyHknxNsC92fHyRW2nbuwMr78pWA4vPIyV6BFG5jS8k2RXEbWiOKNNsw7nWlfT4QMwkEu4uU1vqxsNDtdc1rdrc69aBegyNOQBS+W6aP1ESHp68AoalYKMHKpc+fi00sdQwYU9Y5oW9q4uRX3baAyuGZjP0xuKN3t+T1QnhbhkldP2WP0ooU/VRMhy2rYoE+W6eQRGrghJJG/wWznz5AiPD9EpPST/hoVWOKVco+12IbdILw7yGX2c65xPcLr6obVR+549QrgxU0W02XxS2lXKGc1NT2Zdl6ugh6XpW1RHVz7SjLIZgifwIDAQAB"),
            desc: String::from("Removes advertisements from Finnish websites")
        },
        FilterList {
            uuid: String::from("9852EFC4-99E4-4F2D-A915-9C3196C7A1DE"),
            url: String::from("https://easylist-downloads.adblockplus.org/liste_fr.txt"),
            title: String::from("EasyList Liste FR"),
            langs: [String::from("fr")].to_vec(),
            support_url: String::from("https://forums.lanik.us/viewforum.php?f=91"),
            component_id: String::from("emaecjinaegfkoklcdafkiocjhoeilao"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAsbqIWuMS7r2OPXCsIPbbLG1H/d3NM9uzCMscw7R9ZV3TwhygvMOpZrNp4Y4hImy2H+HE0OniCqzuOAaq7+SHXcdHwItvLKtnRmeWgdqxgEdzJ8rZMWnfi+dODTbA4QvxI6itU5of8trDFbLzFqgnEOBk8ZxtjM/M5v3UeYh+EYHSEyHnDSJKbKevlXC931xlbdca0q0Ps3Ln6w/pJFByGbOh212mD/PvwS6jIH3LYjrMVUMefKC/ywn/AAdnwM5mGirm1NflQCJQOpTjIhbRIXBlACfV/hwI1lqfKbFnyr4aPOdg3JcOZZVoyi+ko3rKG3vH9JPWEy24Ys9A3SYpTwIDAQAB"),
            desc: String::from("Removes advertisements from French websites")
        },
        FilterList {
            uuid: String::from("6C0F4C7F-969B-48A0-897A-14583015A587"),
            url: String::from("https://www.void.gr/kargig/void-gr-filters.txt"),
            title: String::from("Greek AdBlock Filter"),
            langs: [String::from("el")].to_vec(),
            support_url: String::from("https://github.com/kargig/greek-adblockplus-filter"),
            component_id: String::from("pmgkiiodjlmmpimpmphjhkodjnjfkeke"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA4KGRug8Rw1WHk1BPfIdtdw7uwFijUac7jk1lb99lEfSq2uPV2bKCk8lLh/6ahlV/EjSN8mGiFfZIVTDFhuYhVuIO8iETrCZe1ChoI0F8ptHOPQXVPzKUFMpkRqAnH51vqx+3gG78A3+iGfAE+LjerP1j4Jx5jSvTkbN8l+RqKMtjaaL9qRHv3aRQtYB/shGgdxKeOR0f8E6yJ4tIRDHB72bDufN7wbnRoHCNnLkrAPtbIwpWRLKYcOxAB6QqKNCLx/UX/pWpGtyJmMQQBpxQgl3BT8daNp0h4Soc6VPZA9wEIQ5/a/8UpsBT9rwJGj5WdSBPSR8D54aULATPxsienQIDAQAB"),
            desc: String::from("Removes advertisements from Greek websites")
        },
        FilterList {
            uuid: String::from("EDEEE15A-6FA9-4FAC-8CA8-3565508EAAC3"),
            url: String::from("https://raw.githubusercontent.com/szpeter80/hufilter/master/hufilter.txt"),
            title: String::from("Hufilter"),
            langs: [String::from("hu")].to_vec(),
            support_url: String::from("https://github.com/szpeter80/hufilter"),
            component_id: String::from("gemncmbgjgcjjepjkindgdhdilnaanlc"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA4HNXDsDBPP4b/irxacZMYnaPjNMXS31e11nsFBvN9lFOkuwF3bEk9uEk0fDzocF6GSpXbUE0HVTqfKTTnZfvG9m+C3nT8j6N7BB/wST72s0zXCjSlLWJPGmFnFb/EDkFAGmA9FU4C+j28Obehd94OC9pSqu8DYK4LbMWPmk2fgpO9N3ZV/5Y2Ni69WKJwT72prSMzyVVEAYluCYPQWY93g6dJ9RBtwnHCmdK5TG/bN2q6f50Cw/aJSv8nshSdp+KJK6yi6fBOxF5Xb0Bj+xZGC4K4SW9JjElswaGJi2PX5I11w7xC24jNaW6BUHcJ6IXudIVmBFQxWWxkMVwfgqNlwIDAQAB"),
            desc: String::from("Removes advertisements from Hungarian websites")
        },
        FilterList {
            uuid: String::from("93123971-5AE6-47BA-93EA-BE1E4682E2B6"),
            url: String::from("https://raw.githubusercontent.com/heradhis/indonesianadblockrules/master/subscriptions/abpindo.txt"),
            title: String::from("ABPindo"),
            langs: [String::from("id")].to_vec(),
            support_url: String::from("https://github.com/heradhis/indonesianadblockrules"),
            component_id: String::from("egooomckhdgnfbpofhkbhbkiejaihdll"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAptA5jVa5JkYI2jt905om4OLSHGahwgS7tu7GG0sk1YNafOo4ajKrN96Kxj0fgwGrJPhU1UiTDmrgLTZSbuC3hAscbfhuakVNo1pyFfSAVoLWSrOq5l4k6zZK+y1ahxdyJvlbz06RWE6OhIqExxGqLyMjEknkPGxBVO0cKcYHiGYUxvVPxQOg+9fGieXMlSGs/L7Mty1oJOoZ4JcPIFeSvQ5ax48E7l+yAW6psNpPqRAZ5fm7hhZXjd5+3cfXXIMStgX3X0MUHjx2KpYlv3NxMjaZQOAZiuZ3W/H7VWnV7V/ScJ9Eb+e6iG4XS15f7vFQu4zPy4UTYOl6gXnIGWGmsQIDAQAB"),
            desc: String::from("Removes advertisements from Indonesian websites")
        },
        FilterList {
            uuid: String::from("4C07DB6B-6377-4347-836D-68702CF1494A"),
            url: String::from("https://secure.fanboy.co.nz/fanboy-indian.txt"),
            title: String::from("Fanboy's India Filters"),
            langs: [String::from("hi")].to_vec(),
            support_url: String::from("https://www.fanboy.co.nz/filters.html"),
            component_id: String::from("jnnbjhbkmgggeoplhadmppaeddmeapla"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA1KcZJ8dK2ANtw8x19l0g1i2sSsg/G6AJNLORsw+Uz9g6sx7pUJo8TpjyITiwO366C6q/I2KwFpuc0NgGvWs6nLMEX6yaGO8tnrza36Dj7GFTQA9+H0TQxuZdcOfZqP8UakA6exGOl0anzAt/HQB1Gf6ipKilS0M4PAnPwlVaTRmdKxQwqHQrpfWSiKRB3MBG/OT48y9SXKToP+NAIIfVRC7TxPVJ0zIu41UDfU3Mx6zKOBcRfTnhmUT9XIEv2lAfRDyEDXTGg3nSNluZM4Bu4iQKpxj+x7oZDpQMkrpVBZdjpIRMPiD6aMKCo3GULoH15Fb7re5RZLJIxcu0+B8lPwIDAQAB"),
            desc: String::from("Removes advertisements from Hindi websites")
        },
        FilterList {
            uuid: String::from("C3C2F394-D7BB-4BC2-9793-E0F13B2B5971"),
            url: String::from("https://raw.githubusercontent.com/farrokhi/adblock-iran/master/filter.txt"),
            title: String::from("IRN: AdBlock Iran Filter"),
            langs: [String::from("fa")].to_vec(),
            support_url: String::from("https://github.com/farrokhi/adblock-iran"),
            component_id: String::from("dbcccdegkijbppmeaihneimbghfghkdl"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAm3AZE2ne7R55X2j6RxAQHAZKl1hNPgwLOFsYpfAJ6m0uXmKJspguWxatJ9jDBbYLmtXnwX2WORILq1+r4kFtTcN8GNYe7/7o5yDLucI/W9d2vCjmEg95v50MzVQZSwd2gNZVZtL1s0S6pBwX0zI+6kHIFr2xqGV/FNE8L75f30rriQ0xKmenI1OWjyn8gNqIp4mKZW6XxkMRRS9+e0ynDi4ysQA9Ub5YHJxm0t62eqTmIyemgRhP6Rdbi0+GXbqFPjDfC26rtD3wy5f3aYL1V+2ADpdDyCeNlwCH7+vC7LWujqNTgK8wVJ4eH5VbUKC1e9cm/T57OsHJMDC5fbUuswIDAQAB"),
            desc: String::from("Removes advertisements from Persian websites")
        },
        FilterList {
            uuid: String::from("48796273-E783-431E-B864-44D3DCEA66DC"),
            url: String::from("https://adblock.gardar.net/is.abp.txt"),
            title: String::from("Icelandic ABP List"),
            langs: [String::from("is")].to_vec(),
            support_url: String::from("https://adblock.gardar.net/"),
            component_id: String::from("njhlaafgablgnekjaodhgbaomabjibaf"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqSwaNKhg90HCheaJu3sHocbZUjXDs90I0OmijNkDeS291wUjvXAm5YqhNE8aZmPSMVZBjBCwKXrrtTOkMA1b1uBqJ2P83fCZsgNZWbGTD8MorMrU6vyqkWCqLRc+bTTUgzAd55ckUJ/M+HVnjo6QfqUuB3kVzjpwJorQQZUYOLcgDY/Q5/tbrXI5+OGVxAb21pmnk8JHXNNWB2NvpA2o3p0ke/7WEoUH24l91ndOkXkN87eO8rSysl07Eq7gshbednYYiCxRPjuX0aPqbXMYNWXa5NdvIXFJcD2xV/l/QvXRYl+7Ca1igSXaiKc5eJyKSRqY4lf2vG0XCH6VZVxZuQIDAQAB"),
            desc: String::from("Removes advertisements from Icelandic websites")
        },
        FilterList {
            uuid: String::from("85F65E06-D7DA-4144-B6A5-E1AA965D1E47"),
            url: String::from("https://raw.githubusercontent.com/easylist/EasyListHebrew/master/EasyListHebrew.txt"),
            title: String::from("EasyList Hebrew"),
            langs: [String::from("he")].to_vec(),
            support_url: String::from("https://github.com/easylist/EasyListHebrew"),
            component_id: String::from("hjeidaaocognlgpdkfeenmiefipcffbo"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAoUnmZ/fnWAGhAywLBs5IX0OMxK6LOwtjljcwEkt8QD7ZKBekxq+MDrUuRPzav3ky9IyREhXe9F4UWBKPDD8ZQXZ57WQAAMAp3IxbgdAsTqqTEEReUVx+pzjl8lxdp7xEG2gpuM5wq7bjn4zJ3kcdj3vx7bec/YbYf4fV0brQPWghKf2sh3mHXOVh68wEFXYBvcWkGXfuBoRbB9WLflqZYRk3GrLllwBLn1Ag6iuKucvoyv7N23qXKIjqAhyKPmHx4l9w/v2c1pc3NB1af2xvtRWaQp19N98QouFFx5MwAI9+jR77Eox6QvRwA+L9CFkYlDTvT/aS3q+Zb1QH/8AE4QIDAQAB"),
            desc: String::from("Removes advertisements from Hewbrew websites")
        },
        FilterList {
            uuid: String::from("AB1A661D-E946-4F29-B47F-CA3885F6A9F7"),
            url: String::from("https://easylist-downloads.adblockplus.org/easylistitaly.txt"),
            title: String::from("EasyList Italy"),
            langs: [String::from("it")].to_vec(),
            support_url: String::from("https://forums.lanik.us/viewforum.php?f=96"),
            component_id: String::from("nkmllpnhpfieajahfpfmjneipnddhimi"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAurn1cJrIcCa8P7hjGex+OUHi19PRxmjJ5DuQlMAeIaKibwaQOZEPXSvD+O+xgxHZJs1o2DE8zfj6yrAmDfu9+/T0ArT2RWuopDMEfaKdeG0ylHP62WJC+KGUhCiTNmLyPxbU9AiwydVyFOam8vs4Tr+9I3lYKVeClQrtDRM34BTOAsuHRjiuIKoC0jDC2kc+BAsAbzhIdrkEDGD+qx0rCRnGL6c8xODe2PLKSkCSIsqOk44eYOkBqQd0SgmCvQjXS2XczMDNuV7DCZofErsy2iEv/2kzhkkN8GFwbRkYGN9LuK8rtekE34AvZKRHS6e/pHjUCYJb/2xv6elC+VLsJwIDAQAB"),
            desc: String::from("Removes advertisements from Italian websites")
        },
        FilterList {
            uuid: String::from("A0E9F361-A01F-4C0E-A52D-2977A1AD4BFB"),
            url: String::from("https://raw.githubusercontent.com/gioxx/xfiles/master/filtri.txt"),
            title: String::from("ABP X Files"),
            langs: Vec::new(),
            support_url: String::from("https://xfiles.noads.it/"),
            component_id: String::from("agfanagdjcijocanbeednbhclejcjlfo"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAsorIQuFuMI5OaGYaYTu6+kZC4j3qPoWRoD7F9GS0IJC+VEk3XQ7UTsRXlrIxP9obmC7+pByP6hknBUzvKKS9I1v2voIqjUoydWOozbfoVoRhTLN3UiDnoDueXqXiv1MGLzY/ZcsxsxAlIiTcE7+/KdM6pJ72Mn/aLKU3escIJ5E5qOHJOFDLW9587JeWOzexaCOrtiZMclE0KWbUi7qB3Bz3auF6piSzoNGeI1NMwHSSAwhDOQ3UK09aqRKhyfBq6ugrrYyRAr3FWqmMBWkiTsr6SzrbQg3wcGbD+GDvoQmqVf8dH/WYG+srR6PyJdYH5mOQs6Yg+nu1gvwQ46Z74QIDAQAB"),
            desc: String::from("Removes additional advertisements from Italian websites, may break some websites")
        },
        FilterList {
            uuid: String::from("03F91310-9244-40FA-BCF6-DA31B832F34D"),
            url: String::from("https://filters.adtidy.org/ios/filters/7.txt"),
            title: String::from("Adguard Japanese filters (日本用フィルタ)"),
            langs: [String::from("ja")].to_vec(),
            support_url: String::from("https://github.com/AdguardTeam/AdguardFilters"),
            component_id: String::from("ghnjmapememheddlfgmklijahiofgkea"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAsOoGwN4i751gHi1QmHMkFZCXFPseO/Q8qKOQViZI7p6THKqF1G3uHNxh8NjwKfsdcJLyZbnWx7BvDeyUw3K9hqWw4Iq6C0Ta1YEqEJFhcltV7J7aCMPJHdjZk5rpya9eXTWX1hfIYOvujPisKuwMNUmnlpaeWThihf4twu9BUn/X6+jcaqVaQ73q5TLS5vp13A9q2qSbEa79f/uUT8oKzN4S/GorQ6faS4bOl3iHuCT9abVXdy80WSut4bBERKgbc+0aJvi1dhpbCeM4DxVViM2ZccKvxSpyx4NvWj56dNKqFLvzoA4/Chz1udxifIXUHh0701s1Y4fLpY0wWP0uXQIDAQAB"),
            desc: String::from("Removes advertisements from Japanese websites")
        },
        FilterList {
            uuid: String::from("45B3ED40-C607-454F-A623-195FDD084637"),
            url: String::from("https://raw.githubusercontent.com/yous/YousList/master/youslist.txt"),
            title: String::from("YousList"),
            langs: [String::from("ko")].to_vec(),
            support_url: String::from("https://github.com/yous/YousList"),
            component_id: String::from("djhjpnilfflibdflbkgapjfldapkjcgl"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAux80m8cYDEXwq+nMwmui6NCO9SFAdcGly5eq4uGEIQNB1R6Tr9JMqosHLZ4PnaUJqJwFfLWfmxzXj3q0DIpqqpdSq/jTYT/MvOldC+VQFO+NIjXhtysh4Z5F0BzlsQx/leMnV6yoyQjBX53n9cl3BvQK/EdbuQSDiNqX2TSVLm7hnr7Vf8m4XYRSCSJybY/1Tk3Cqgqywlkr+YN58L1/txXCQ9LJ5SxJ9I56TxqA1uT97hBmQikvnopuLh1SovDfjtCZwWwaGDD4ujW+Qaeh9dRrojS47iwG/Twu1xbb7ra8cn8BxdzsPjUSSurpPz/9sUooYOGJO44p7u77sxeTXQIDAQAB"),
            desc: String::from("Removes advertisements from Korean websites")
        },
        FilterList {
            uuid: String::from("51260D6E-28F8-4EEC-B76D-3046DADC27C9"),
            url: String::from("https://www.fanboy.co.nz/fanboy-korean.txt"),
            title: String::from("Fanboy's Korean"),
            langs: Vec::new(),
            support_url: String::from("https://forums.lanik.us/"),
            component_id: String::from("oidcknjcjepjgfpammgdalpnjefekhge"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAvCOljWKWLWfq/k/BE9gIZtI1MstmG+NcgGBGAP0R7xgaUMU5phdSbQf83Zt9ctwDRpisHWlGS6o+tk93zMIoJVj6RMQ2Zee6QPAKAGgwuCXF7A/ciI3lRyX7ts49XV8GAbasu1mBHntz+GpmOVmoiRxcDMUDDEqsSXgckCM9HkYvIyHQWyEgeulKdhQ2HoCptD2Wgmws6NzRTgQ94+DHu2o6J4MsG74h7L/cG3XB8WQNuqlpjjFIQTXftuUWDSkyR3tlmMxGN1PXAH6RZBNmwQTwdgrOAqEup82dWaO3BqoYGZdYeRaUGRc73iPdvvjZb1tvmqLdVSq7Ur1XJjJJTwIDAQAB"),
            desc: String::from("Removes additional advertisements from Korean websites, may break some websites")
        },
        FilterList {
            uuid: String::from("4E8B1A63-DEBE-4B8B-AD78-3811C632B353"),
            url: String::from("https://raw.githubusercontent.com/EasyList-Lithuania/easylist_lithuania/master/easylistlithuania.txt"),
            title: String::from("Adblock Plus Lithuania"),
            langs: [String::from("lt")].to_vec(),
            support_url: String::from("https://github.com/EasyList-Lithuania/easylist_lithuania"),
            component_id: String::from("ekodlgldheejnlkhiceghfgdcplpeoek"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA5dB+7xR4lcPQCW84V4zhLiYhAvKxgdo2/cze+C8E3+ye1AO+a1CWbdPgft36vtTm4nkDzyC3P9O/aEU8jxShKEU1DDk8YBdRnvctQ9PPvwNyeS9LCYeT5a9crE9M/Z+kaFyq0SRe5cpowOBG8x4OYTt9Y7L9whEGzZYRZlgklli1AES6e2B9XUAdHXV/wHsaf2FrdPFtDfZZEFdr60edk4f0iGppiwkaGJiOWVF1ya47NoSMl4fIF7Klw9OkfKLJHjk9YXZmXCfqxQl8FnBFe/SzbSTVCAhdaggQAwG4VmojjMrBHcQl0VJDmpoY2jFZkiO3GLmAZCYIYaN1tFA8ZwIDAQAB"),
            desc: String::from("Removes advertisements from Lithuanian websites")
        },
        FilterList {
            uuid: String::from("15B64333-BAF9-4B77-ADC8-935433CD6F4C"),
            url: String::from("https://notabug.org/latvian-list/adblock-latvian/raw/master/lists/latvian-list.txt"),
            title: String::from("Latvian List"),
            langs: [String::from("lv")].to_vec(),
            support_url: String::from("https://notabug.org/latvian-list/adblock-latvian"),
            component_id: String::from("hmabmnondepbfogenlfklniehjedmicd"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAst1posKDpKt3WLU07CziowQnBKYXzH2i/sDdJfMuTcKSNQvn9dxbHVLhg8Ih7NmN6SSJTRgb2PughdVNPXqlT3/jGioDC0gN8kBrBoN2YWgIW2wdvTCPvBOfwTOhGueQY6AtE7zD/3m9v6Wfcw07Rj84Su0qI1Zadmq2pBWo5z82vOAI2yV83YGDbnyK1JaFeLToYQmj+bMEojoZ4Lk4PbFmopVh1GkeOdCKtVN2NTIy43N/w0tS0wlLxjwTyZ6RIcK3VOhQXBqcpwKpKm/4WDksTvNRLZ8e526z/nqaasM/meS22hURh6NPtIOdy6/TspTzFPiRdj2xgNfQZ9oRxwIDAQAB"),
            desc: String::from("Removes advertisements from Latvian websites")
        },
        FilterList {
            uuid: String::from("9D644676-4784-4982-B94D-C9AB19098D2A"),
            url: String::from("https://easylist-downloads.adblockplus.org/easylistdutch.txt"),
            title: String::from("EasyList Dutch"),
            langs: [String::from("nl")].to_vec(),
            support_url: String::from("https://forums.lanik.us/viewforum.php?f=100"),
            component_id: String::from("fbmjnabmpmfnfknjmbegjmjigmelggmf"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAqqfwmNS4XOq9pWC3XSMt5WcqoKaj3lRpYAwZKTP+6DwA9pG+Zw+0iWC95riSLjqPgX+0d2cZaqjinuNn3mUMOeGdbwSIeRLE50J5J/dMmkg5YO09orZKLBjMfJG5IDgfXdZLSJtmzKC4Xj2y6KSuQ7N0Sg5f1Ecc19nFbcFazCaIhKvcoA84J7Twf2IoCDuPMsGplgZCBtFQkKeqILaVhJZeD0my6pdC2KJREbM3eRnntE44O0sbmemCfHs9BV50hVb913zGDZ379eTqg3mPjvH+VnY+7RvjVPayJP4+51zRJYKi18W7KMry3sj4ZZ3EyNKmbwlGQOzAyd/Qtj4I3wIDAQAB"),
            desc: String::from("Removes advertisements from Dutch websites")
        },
        FilterList {
            uuid: String::from("BF9234EB-4CB7-4CED-9FCB-F1FD31B0666C"),
            url: String::from("https://raw.githubusercontent.com/MajkiIT/polish-ads-filter/master/polish-adblock-filters/adblock.txt"),
            title: String::from("Oficjalne Polskie Filtry do AdBlocka, uBlocka Origin i AdGuarda"),
            langs: [String::from("pl")].to_vec(),
            support_url: String::from("https://github.com/MajkiIT/polish-ads-filter/issues"),
            component_id: String::from("paoecjnjjbclkgbempaeemcbeldldlbo"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAsUqWP6CeMx79UyZ3GZ1XcBGexIgml00sB286wZ7dJsfqG7oI0EGRoqrDeRreYcOTl+HvXsRJvR1FfkKJzD5svdhR4mn4lI+FXUDCvgEZ9CFa0YfASuoTIrdZtG74Twu2ai52ZJzrQ9ike97bdwzuZo+uymw26S+5/+IQbriIYoxEbJd7EryZuo+W65LdSat/NOKKf1QnVTIOoqMrXiewRYywnmZATfDIi0uKXuQfF15lbNBkQllmPH1xlMkz2WnvSvqI4HKPAmEFJWVUkiNhGKFZkTk1+88CgGGPVsKllxLaDOD+j8Kb0+h44RxObHTF/vFkfh8FfzujFj3HtevjCQIDAQAB"),
            desc: String::from("Removes advertisements from Polish websites"),
        },
        FilterList {
            uuid: String::from("867EF333-8336-455C-9CC6-98749AEE69E4"),
            url: String::from("https://raw.githubusercontent.com/olegwukr/polish-privacy-filters/master/anti-adblock.txt"),
            title: String::from("Oficjalne Polskie Filtry Przeciwko Alertom o Adblocku"),
            langs: [String::from("pl")].to_vec(),
            support_url: String::from("https://github.com/olegwukr/polish-privacy-filters/issues"),
            component_id: String::from("baophminpaegfihdcekehejfhpmjimle"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEApX6GHwd6ZsPNk4iomzHF6fb69FJcVyRNTQc3X/LlDuEXERJ/eZzDVMn2pCm2CTCXQHweQWqBkC/20FkjniwGb9LSjzP5jdcDCFmSwaFWdiM7xG+BfMFP+XDJtjOlqirWESi6dzwnQ5pKQDpNCblMBuuhT1WyDLtHODwbNJs/jILdSAapW8eQApQ/iCGidYPbPvPL53bq+u45UXXljillsJTbGV8vu2VVhf9/fL5McKu7uX6xR2i4WR2x1hQYMYu5rnFIrDNWGIn4CNDodO22nyBBjznGfQ8XVp558s5tC+v+12hY6HJW4CWJ3Oes+PXuLPDUwYuJKkuncfADk49oVQIDAQAB"),
            desc: String::from("Defuses anti-adblock from Polish websites"),
        },
        FilterList {
            uuid: String::from("CB3A9B4A-C9F3-40FA-A6B8-5219ED5FA9ED"),
            url: String::from("https://raw.githubusercontent.com/olegwukr/polish-privacy-filters/master/anti-adblock-suplement.txt"),
            title: String::from("Oficjalne Polskie Filtry Przeciwko Alertom o Adblocku - Uzupełnienie"),
            langs: [String::from("pl")].to_vec(),
            support_url: String::from("https://github.com/olegwukr/polish-privacy-filters/issues"),
            component_id: String::from("ndgeclhidhlfgmjdcapejaldbahmkgbi"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAvZaLRYXrVyL6jjJF8guXpjmUFv3nJd8hfLmbldBIiJ21bMPpaypGBjQANxIU1Sfz1jpy9J+OkB1ifoDg0ScWqDCD0zpjjS87g9ANGantkh55Y+TYDe7yGq6JF2ELr618y3UJTSyfMUi2jLlmved/1Zmtuup2+nWS3Od6NfnXmV+pHJXJLTX7n397RVb1RNN8U5WIvx6vnpZPVB2H8YoNJd9JMj2olIm6yt4Y0ODMOOAXuROz02QLBwnlZC39Z+BuNVxW2fqhLqFw28MD308v2uYiY/Vc0enna8UISSvebYwJedwZFCzk1CVWaO0Y6vHOBVtH4DwHb5sVxUzx/KI3dwIDAQAB"),
            desc: String::from("Defuses anti-adblock from Polish websites"),
        },
        FilterList {
            uuid: String::from("AD3E8454-F376-11E8-8EB2-F2801F1B9FD1"),
            url: String::from("https://raw.githubusercontent.com/tcptomato/ROad-Block/master/road-block-filters-light.txt"),
            title: String::from("Romanian Ad (ROad) Block List Light"),
            langs: [String::from("ro")].to_vec(),
            support_url: String::from("https://github.com/tcptomato/ROad-Block"),
            component_id: String::from("hojdjlebfkngledgkgecohjkjjojaekd"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAnhLXi2u795hnBUJi/vS8qtMGoTYk9NRefWk+SI6fkqVdiKs/eM6Y8v3To4HgNmtYb4jAoYctcq3/CS3hzGCLEwQbDuL8Y8UP5B6PWgzuiRrAobRl1DtXO1+Q57VIrYTpJVLCqaTulclys7Fka/wD5o78Y0vAfSenBZTzRUXwTZd9Z7SRNwcJyccbi7zL8UDWnJMBnD/dnV7t2q41MHiCgdzimOSuoRZmTBrupVc0QYhqoxy6ePkHFDGL2U25omAZckkzpQbtvJEE2lmg7YqnaSvGDzsmqd+j7hVWjpm/ncArLOWBCbER3MdHwFeOI2rFJWcO7GY5etQsA5128FAv0wIDAQAB"),
            desc: String::from("Removes advertisements from Romanian websites"),
        },
        FilterList {
            uuid: String::from("80470EEC-970F-4F2C-BF6B-4810520C72E6"),
            url: String::from("https://easylist-downloads.adblockplus.org/advblock.txt"),
            title: String::from("RU AdList (Дополнительная региональная подписка)"),
            langs: [String::from("ru"), String::from("uk"), String::from("be")].to_vec(),
            support_url: String::from("https://forums.lanik.us/viewforum.php?f=102"),
            component_id: String::from("enkheaiicpeffbfgjiklngbpkilnbkoi"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEArVVgKRE868yup0HfX4+HyZmJVIk33AKivwvRRfjHRxeC+lLnRjNiY0LKS/K65J6SNLgUsZGfT5u4h4F423O/pbZl6zdfs5kOyStlmLPXhFtF/bIXIsUtdJ0R3dEz+nSg0C2L/FnE5Qr8M4thdmq/DIP1C70mj8pCnX1939hXyR0ymQkYp573O+LJ0q1L41jBqHzNKWngfBc79I2Kbt1pLluBT2X7zZVbb+1ap3Ad/VMeFDB2yurRs88cYJZOal7mgTgI/Zkuzsh2Dnql5+UNOCHinYjcOvUifGgkdsJIJxL57PxRzbriLCNjShoOV3Fpc0XYL1KSWvIVuW0bYeLmrwIDAQAB"),
            desc: String::from("Removes advertisements from Russian websites"),
        },
        FilterList {
            uuid: String::from("1088D292-2369-4D40-9BDF-C7DC03C05966"),
            url: String::from("https://adguard.com/en/filter-rules.html?id=1"),
            title: String::from("Adguard Russian Filter"),
            langs: Vec::new(),
            support_url: String::from("https://forum.adguard.com/forumdisplay.php?69-%D0%A4%D0%B8%D0%BB%D1%8C%D1%82%D1%80%D1%8B-Adguard"),
            component_id: String::from("dmoefgliihlcfplldbllllbofegmojne"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA4p+4n2wNFQCqQBBJDsvs+oqNYGzX3cbpY7fKbCjrRVE5esJK5HZJDoUUg43pPvKrCOIQ+lF+dXpBaCNnO4O/7JeFt2IFRJnKhE3ipIBAAbFymfo5T2uWFdyh6HcK0FNyJ/7FyHnANe7vYhXJS1Fqmh6jTYkAEIbrbmxtzrDMefx3XJcVhUV3XAPlP+K3MerxudIH++4fn3X0vKob5oQQQ9ZZ1PVcW6ZdZTQwQWtaVDb6prT+ULaphRRmnZpZuRXyHMv9KC8YP3K5ou+/Yd3uxxMwKmJXD67ZoNMtS/Dtr0btQsLxiEgox5Swd4iqyLM/SMxr3LqgUIlNwn7KRbMnZwIDAQAB"),
            desc: String::from("Removes additional advertisements from Russian websites, may break some websites")
        },
        FilterList {
            uuid: String::from("DABC6490-70E5-46DD-8BE2-358FB9A37C85"),
            url: String::from("https://easylist-downloads.adblockplus.org/bitblock.txt"),
            title: String::from("BitBlock List (Дополнительная подписка фильтров)"),
            langs: Vec::new(),
            support_url: String::from("https://forums.lanik.us/viewforum.php?f=102"),
            component_id: String::from("fmcofgdkijoanfaodpdfjipdgnjbiolk"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEApGBuuxC9pqx2s5hUd3K8zeFUtMFt9X+rSKR3elqWztjIQrbOdXSeezHvhAUdzgc+Wv79ZoRf4i4amYs3Mg3wg783BAqLvlu9r6FsUAbcgVQtt+MT3Z4ZepwvzWU0NjUd1q4O2pNEUsE8SPjmOeb3KHOF5WX7CA1uIHT5xGQsU5Uh3VTZC8FIOGjCskDAAnJGUeOowlMBGL2UvlNQLiqzPSvI9byjwxIMN5OfCmxXXr4R9m88oVK2D1gj7vfwBVJcRdV8ner4ZSuT68ncSyaQRtgI3/QyHc0J6giCRFmF0bHN/5kjFIWrHg5+uiBQN4Qt39TVCUU024Fi2RGInvTTdQIDAQAB"),
            desc: String::from("Removes additional advertisements from Russian websites, may break some websites")
        },
        FilterList {
            uuid: String::from("AE657374-1851-4DC4-892B-9212B13B15A7"),
            url: String::from("https://easylist-downloads.adblockplus.org/easylistspanish.txt"),
            title: String::from("EasyList Spanish"),
            langs: [String::from("es")].to_vec(),
            support_url: String::from("https://forums.lanik.us/viewforum.php?f=103"),
            component_id: String::from("pdecoifadfkklajdlmndjpkhabpklldh"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA2eGyWcTM6Cpmkw6CBBxQbJCgp3Q4jyh+JR/Aqq5G+OFzxFpwlqW0dH9kNuUs30iSt1tt1gMZGYnhPKiGhtX3nV1iYg2K8k82wNqA5+ODfHxnnVn536UoC7rmjXL+mhpymxgkjGCQ+1HVmnCcSC9mxTPy65ihor+YZcRRPo0IhjQTx3NgdpzkGYvpQVjwnw3a5FpRBCbbp3X2x3EGV3DcjvT6DvvxSU/mAUPlXISo9OFHYUpADilqAevXQIs49LSmefSDu4pezGyR/JoRLh7QR4N3fC17V2E0GazWxvn2U985hPE3tvFcH+LM3EypVRCl6E9AiUZCeumqMBffyXw1AwIDAQAB"),
            desc: String::from("Removes advertisements from Spanish websites")
        },
        FilterList {
            uuid: String::from("1FEAF960-F377-11E8-8EB2-F2801F1B9FD1"),
            url: String::from("https://filters.adtidy.org/extension/ublock/filters/9.txt"),
            title: String::from("Adguard Spanish/Portuguese"),
            langs: [String::from("es"), String::from("pt")].to_vec(),
            support_url: String::from("https://github.com/AdguardTeam/AdguardFilters#adguard-filters"),
            component_id: String::from("jpolmkeojnkicccihhepfbkhcbicimpa"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAs1E/bf3s1EQeJY10IT5/ZMCzfMAm6SKyWUCeHBkWZLcfwyYLJww84EC2jCLeYgwukOmZjwtnDrasVhUyKOif7dKIBEZizsvSldi8tzHqTbX3PFKsLhRXCETbU0kkOlArGRGLaBIhgT07qPtlehYCZoDdowk42025fVtfVEMtZg8yBIqtFT/bDL96lRDQIW+1uAM3uFkzvRtQgsYhoI6JlyqFw6fqowRx8a+zHvtQzAUyIaTGf0OFEHwGCFlHXmTYpcOXlcUAXn4RnJvx+thpeDBtAvT6LubTLNQClBbwjGL5d7NlGNPByYdcZZcvGPmBWX/vnobY5QGP4lWxZvWfFwIDAQAB"),
            desc: String::from("Removes advertisements from Spanish and Portuguese websites")
        },
        FilterList {
            uuid: String::from("418D293D-72A8-4A28-8718-A1EE40A45AAF"),
            url: String::from("https://raw.githubusercontent.com/betterwebleon/slovenian-list/master/filters.txt"),
            title: String::from("Slovenian List"),
            langs: [String::from("sl")].to_vec(),
            support_url: String::from("https://github.com/betterwebleon/slovenian-list"),
            component_id: String::from("lddghfaofadfpaajgncgkbjhalgohfkd"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA4cRSF2Rg5SSG6mwE6NQCnX4a0MfzC9URqNFnI4Wf3d3a7CkhmVeNZHxSCGGLxNV9SjCi5tko7NdMqIwnN/vliZV+jnEDi8Lj+zz9nftkaGXe3jNoP7tr/+Qkqphc76j3wIpsQx/vBnfVTn5lrNynZL6qpFzX5dj4ukdJ6BOx1YTNdJV9LOyMWbC5rno1mpd14aS7R2T6xfnm3+nupaZMAbUeN/1bwxDdND/mbjFzFvkPCC+4m758tI/5kSJOefy8kNvp9BM64LXPA4sF59ttJtCIOJDAyhM1P0Danyze2g/0GGnojDuzZilfeSCeEpDsc+S78Tyqz/lMtxt2LZkvoQIDAQAB"),
            desc: String::from("Removes advertisements from Slovenian websites")
        },
        FilterList {
            uuid: String::from("7DC2AC80-5BBC-49B8-B473-A31A1145CAC1"),
            url: String::from("https://raw.githubusercontent.com/lassekongo83/Frellwits-filter-lists/master/Frellwits-Swedish-Filter.txt"),
            title: String::from("Frellwit's Filter List"),
            langs: [String::from("sv")].to_vec(),
            support_url: String::from("https://github.com/lassekongo83/Frellwits-filter-lists"),
            component_id: String::from("oimfmeehpinnecjghphifehbbnddjkmf"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA17Vf1qj8dWwYVtGpBHWc9gLiITU1XrTnb1sDASIeuKYp9JNBtEnBwy4oBlOoZd2uWFKxXrRtaimdwqa627gi9DB17t/RgzisXSpLubXbVVelRWllaX26SioGxsGcQhS2/e1Bc0inQ8GODM6mk5FPZ9RObFN1N/QVz35anN4VNcjtETD/XpujYXE1BU3C0KGBlWwc+cQZ6sGojWEPrb7aRXSTJ5y/ugwGomTTpbT+Jt9nFrMfuAmJHvWS0Ev96dDmn1zsuoPGUExVFjGBunphRYMVCg9LUGzY0FN5+dp6fljrTJrtUOEfvh40vmjahKd0w6bKpgTAOUEaWulmVSr37QIDAQAB"),
            desc: String::from("Removes advertisements from Swedish websites")
        },
        FilterList {
            uuid: String::from("658F092A-F377-11E8-8EB2-F2801F1B9FD1"),
            url: String::from("https://raw.githubusercontent.com/easylist-thailand/easylist-thailand/master/subscription/easylist-thailand.txt"),
            title: String::from("EasyList Thailand"),
            langs: [String::from("th")].to_vec(),
            support_url: String::from("https://github.com/easylist-thailand/easylist-thailand"),
            component_id: String::from("jplgiejfnpolnfnigblbfeeidoimingd"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAy4EQ0IdWDAAqRT6ymtxWG32Whfmv5TMxKuEP3rs9LMfa+iU+xiyE/XvWjgA58n4Bs2vQgefCoqY+a9B0TUkqz0nDcrBi371w1EZSxNRySslO5VvLHdRdYwynTMwDsAlHIPZ6pgh9zwprW1Lxz31CC2EHJeBGmBQ/S8My/VRiN8Y2Jj7yZDX1rTvBrYPj5XAwe2MAPAsMJD4lHcx7uClEbVq/4AxNpmNay5kamFaX6qt8/765RyPYuqgneharP7EJ9HToH56l/KR7doOywTyVPQYvEhD+a1mioMfEtYNxvqY4lKDhctTV8aU7RItAgwGTW+msldvdPfs3QWV5o7yrtQIDAQAB"),
            desc: String::from("Removes advertisements from Thai websites")
        },
        FilterList {
            uuid: String::from("1BE19EFD-9191-4560-878E-30ECA72B5B3C"),
            url: String::from("https://adguard.com/filter-rules.html?id=13"),
            title: String::from("Adguard Turkish Filter"),
            langs: [String::from("tr")].to_vec(),
            support_url: String::from("https://forum.adguard.com/forumdisplay.php?51-Filter-Rules"),
            component_id: String::from("oooemoeokehlgldpjjhcgbndjcekllim"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA2We4hmp3TwsrKyOb6rF/mCjy9TW3w9n9CD1rZMXUF3U6CCgxH4lps5HiLlxUFaIhhcUEXrGlXbk4TE2LlTv4VS53O23YixZXQ/xMmpWSyBvc3/jBCrAAcvDLAZY53J1T/9t7DNZdpXkX3rNpYB4L5/5dyzQI+sZZoTBe5dLyJOR1uDZJphpXRWSKqBRLn4SJ5uOGgtqG5J4rMhB+SUrNhWs8AyM8+tdoaxOjx7n+PA2Rx7/foty1Bbd7Hfc1Eg0C9R40inJNgH+IDxZ07ZFqiAuY1Z16lr4bwunk7ft4tTafci0M2t86JkoH0B4yiTBKthB6AkmZ0/dejeQeOBszYQIDAQAB"),
            desc: String::from("Removes advertisements from Turkish websites")
        },
        FilterList {
            uuid: String::from("6A0209AC-9869-4FD6-A9DF-039B4200D52C"),
            url: String::from("https://raw.githubusercontent.com/abpvn/abpvn/master/filter/abpvn.txt"),
            title: String::from("ABPVN List"),
            langs: [String::from("vi")].to_vec(),
            support_url: String::from("https://abpvn.com/"),
            component_id: String::from("cklgijeopkpaadeipkhdaodemoenlene"),
            base64_public_key: String::from("MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAymFhKEG/UJ8ZyKjdx4xfRFtECXdWXixG8GoS3mrw/haeVQoB1jXmPBQTZfL2WGZqYvrAkHRRel7XEoZNYziP3bCYbS4yVqKnDUp1u5GIsMsN0Pff1O1SHEbqClb79vAVhftNq1VQkHPpXQdoSiINQ12Om8WbOIuaNxkrTToFW7XRMtbI3tluoLUSy9YTkCEGah68Dl1uL6nOzOxaMV1iQRRk5Pw4ugTzwGHHL2U2kDYDNrlywK8cUIFgtZskqQ/TF1zF6u9xTGjwjB9X319XrTg2llcojCgj/dllBuXL2aJoDsS3qAVzqbSYxIE6bQU8JX8wv+KCDMpJt/dHPQqOMwIDAQAB"),
            desc: String::from("Removes advertisements from Vietnamese websites")
        }
    ].to_vec()
}
