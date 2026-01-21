/// <reference lib="es2020" />

declare interface MimeType {
    mime: string;
}

export interface Resource {
    name: string;
    aliases: string[];
    kind: MimeType | "template";
    content: string;
    dependencies?: string[];
    permission?: number;
}

export interface FilterListMetadata {
    homepage?: string;
    title?: string;
    expires?: number;
    redirect?: string;
}

export interface ParseOptions {
    format: typeof FilterFormat;
    rule_types: typeof RuleTypes;
}

declare interface BlockerResult {
    matched: boolean,
    important: boolean,
    redirect?: string,
    rewritten_url?: string,
    exception?: string,
    filter?: string,
}

export class Engine {
    constructor(rules: FilterSet, debug: boolean);
    addResource(resource: Resource): boolean;
    check(url: string, source_url: string, request_type: string, debug?: false): boolean;
    check(url: string, source_url: string, request_type: string, debug: true): BlockerResult;
    clearTags(): null;
    deserialize(serialized_handle: ArrayBuffer): null;
    enableTag(tag: string): null;
    getResource(name: string): Resource;
    serializeCompressed(): ArrayBuffer;
    serializeRaw(): ArrayBuffer;
    tagExists(tag: string): boolean;
    useResources(resources: Resource[]): null;
}

type CbType =
    'block' |
    'block-cookies' |
    'css-display-none' |
    'ignore-previous-rules' |
    'make-https';

declare interface CbAction {
    type: CbType,
    selector?: string,
}

type CbResourceType =
    'document' |
    'image' |
    'style-sheet' |
    'script' |
    'font' |
    'raw' |
    'svg-document' |
    'media' |
    'popup';

type CbLoadType =
    'first-party' |
    'third-party';

declare interface CbTrigger {
    'url-filter': string,
    'url-filter-is-case-sensitive'?: boolean,
    'if-domain'?: string[],
    'unless-domain'?: string[],
    'resource-type'?: Record<string, CbResourceType>,
    'load-type'?: CbLoadType[],
    'if-top-url'?: string[],
    'unless-top-url'?: string[],
}

declare interface CbRule {
    action: CbAction,
    trigger: CbTrigger,
}

declare interface ContentBlockingConversionResult {
  content_blocking_rules: CbRule[],
  filters_used: string[],
}

export class FilterSet {
    constructor(debug: boolean);
    addFilter(filter: string, opts?: ParseOptions): null;
    addFilters(rules: string[], opts?: ParseOptions): FilterListMetadata;
    intoContentBlocking(): ContentBlockingConversionResult | undefined;
}

export const FilterFormat: {
    HOSTS: string;
    STANDARD: string;
};

export const RuleTypes: {
    ALL: string;
    COSMETIC_ONLY: string;
    NETWORK_ONLY: string;
};

export function uBlockResources(
    web_accessible_resource_dir: string,
    redirect_resources_path: string,
    scriptlets_path?: string,
): Resource[];
