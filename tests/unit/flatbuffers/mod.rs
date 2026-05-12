use crate::filters::flatbuffer_generated::fb::NetworkFilterArgs;
use crate::filters::network::NetworkFilterMask;

/// If this ever changes, confirm that the default value hardcoded in `fb_network_filter.fbs`
/// continues to track the most common mask.
#[test]
fn default_network_filter_mask() {
    assert_eq!(
        NetworkFilterArgs::default().mask,
        (NetworkFilterMask::DEFAULT_OPTIONS
            | NetworkFilterMask::IS_HOSTNAME_ANCHOR
            | NetworkFilterMask::IS_RIGHT_ANCHOR
            | NetworkFilterMask::FROM_DOCUMENT)
            .bits()
    );
}
