extern crate criterion;

use criterion::*;

use adblock::utils::rules_from_lists;
use adblock::lists::parse_filters;
use adblock::cosmetic_filter_cache::CosmeticFilterCache;

fn by_hostname(c: &mut Criterion) {
    c.bench(
        "cosmetic hostname match",
        Benchmark::new("easylist", move |b| {
            let rules = rules_from_lists(&vec![
                "data/easylist.to/easylist/easylist.txt".to_owned(),
            ]);
            let (_, cosmetic_filters) = parse_filters(&rules, false, true, false);
            let cfcache = CosmeticFilterCache::new(cosmetic_filters);
            b.iter(|| cfcache.hostname_stylesheet("google.com"))
        }).with_function("many lists", move |b| {
            let rules = rules_from_lists(&vec![
                "data/easylist.to/easylist/easylist.txt".to_owned(),
                "data/easylist.to/easylistgermany/easylistgermany.txt".to_owned(),
                "data/uBlockOrigin/filters.txt".to_owned(),
                "data/uBlockOrigin/unbreak.txt".to_owned(),
            ]);
            let (_, cosmetic_filters) = parse_filters(&rules, false, true, false);
            let cfcache = CosmeticFilterCache::new(cosmetic_filters);
            b.iter(|| cfcache.hostname_stylesheet("google.com"))
        }).with_function("complex_hostname", move |b| {
            let rules = rules_from_lists(&vec![
                "data/easylist.to/easylist/easylist.txt".to_owned(),
                "data/easylist.to/easylistgermany/easylistgermany.txt".to_owned(),
                "data/uBlockOrigin/filters.txt".to_owned(),
                "data/uBlockOrigin/unbreak.txt".to_owned(),
            ]);
            let (_, cosmetic_filters) = parse_filters(&rules, false, true, false);
            let cfcache = CosmeticFilterCache::new(cosmetic_filters);
            b.iter(|| cfcache.hostname_stylesheet("ads.serve.1.domain.google.com"))
        })
        .throughput(Throughput::Elements(1))
        .sample_size(20)
    );
}

fn by_classes_ids(c: &mut Criterion) {
    c.bench(
        "cosmetic class, id match",
        Benchmark::new("easylist", move |b| {
            let rules = rules_from_lists(&vec![
                "data/easylist.to/easylist/easylist.txt".to_owned(),
            ]);
            let (_, cosmetic_filters) = parse_filters(&rules, false, true, false);
            let cfcache = CosmeticFilterCache::new(cosmetic_filters);
            b.iter(|| cfcache.class_id_stylesheet(&vec!["ad".to_owned()][..], &vec!["ad".to_owned()][..]))
        }).with_function("many lists", move |b| {
            let rules = rules_from_lists(&vec![
                "data/easylist.to/easylist/easylist.txt".to_owned(),
                "data/easylist.to/easylistgermany/easylistgermany.txt".to_owned(),
                "data/uBlockOrigin/filters.txt".to_owned(),
                "data/uBlockOrigin/unbreak.txt".to_owned(),
            ]);
            let (_, cosmetic_filters) = parse_filters(&rules, false, true, false);
            let cfcache = CosmeticFilterCache::new(cosmetic_filters);
            b.iter(|| cfcache.class_id_stylesheet(&vec!["ad".to_owned()][..], &vec!["ad".to_owned()][..]))
        }).with_function("many matching classes and ids", move |b| {
            let rules = rules_from_lists(&vec![
                "data/easylist.to/easylist/easylist.txt".to_owned(),
                "data/easylist.to/easylistgermany/easylistgermany.txt".to_owned(),
                "data/uBlockOrigin/filters.txt".to_owned(),
                "data/uBlockOrigin/unbreak.txt".to_owned(),
            ]);
            let (_, cosmetic_filters) = parse_filters(&rules, false, true, false);
            let cfcache = CosmeticFilterCache::new(cosmetic_filters);
            let class_list = vec![
                "block-bg-advertisement-region-1".to_owned(),
                "photobox-adbox".to_owned(),
                "headerad-720".to_owned(),
                "rscontainer".to_owned(),
                "rail-article-sponsored".to_owned(),
                "fbPhotoSnowboxAds".to_owned(),
                "sidebar_ad_module".to_owned(),
                "ad-728x90_forum".to_owned(),
                "commercial-unit-desktop-rhs".to_owned(),
                "sponsored-editorial".to_owned(),
                "rr-300x600-ad".to_owned(),
                "adfoot".to_owned(),
                "lads".to_owned(),
            ];
            let id_list = vec![
                "footer-adspace".to_owned(),
                "adsponsored_links_box".to_owned(),
                "lsadvert-top".to_owned(),
                "mn".to_owned(),
                "col-right-ad".to_owned(),
                "view_ads_bottom_bg_middle".to_owned(),
                "ad_468x60".to_owned(),
                "rightAdColumn".to_owned(),
                "content".to_owned(),
                "rhs_block".to_owned(),
                "center_col".to_owned(),
                "header".to_owned(),
                "advertisingModule160x600".to_owned(),
            ];
            b.iter(|| cfcache.class_id_stylesheet(&class_list[..], &id_list[..]))
        })
        .throughput(Throughput::Elements(1))
        .sample_size(20)
    );
}

criterion_group!(
  cosmetic_benches,
  by_hostname,
  by_classes_ids,
);
criterion_main!(cosmetic_benches);
