@0x986b3393db1396c9;

struct NetworkFilter {
  mask @0 :UInt32;  # NetworkFilterMask (network.rs)

  # These arrays contain sorted (ascending) indices in the |uniqueDomainsHashes|
  # instead of the hashes themselves. This approach saves memory, as there
  # typically aren't many unique hashes
  optDomains @1 :List(UInt16);
  optNotDomains @2 :List(UInt16);

  patterns @3 :List(Text);
  modifierOption @4 :Text;
  hostname @5 :Text;

  # Optional fields that are often empty - only store if non-empty
  tag @6 :Text;
  rawLine @7 :Text;
}

struct NetworkFilterList {
  networkFilters @0 :List(NetworkFilter);
  uniqueDomainsHashes @1 :List(UInt64);
}
