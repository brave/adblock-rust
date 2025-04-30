#!/usr/bin/env node

const { text } = require('node:stream/consumers')

const { ArgumentDefaultsHelpFormatter, ArgumentParser, FileType } = require('argparse')

const adblockRust = require('./index.js')
const adblockRustPackage = require('./../package.json')

// These are defined by different content filter projects (AdBlock Plus,
// uBlockOrigin, AdGuard, etc.).
// For example, https://github.com/gorhill/uBlock/wiki/Static-filter-syntax
const filterListRequestTypes = [
  'beacon',
  'csp_report',
  'document',
  'font',
  'image',
  'media',
  'object',
  'ping',
  'script',
  'stylesheet',
  'sub_frame',
  'websocket',
  'xhr',
  'other',
  'speculative',
  'web_manifest',
  'xbl',
  'xml_dtd',
  'xslt'
]

// These values are defined by Blink, in `Resource::ResourceTypeToString`.
// See third_party/blink/renderer/platform/loader/fetch/resource.h.
// The OTHER catch all case covers the additional types
// defined in `blink::Resource::InitiatorTypeNameToString`.
//
// See https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/platform/loader/fetch/resource.cc
/* eslint-disable quote-props */
const chromiumRequestTypeMapping = {
  'Attribution resource': 'other',
  'Audio': 'media',
  'CSS resource': 'stylesheet',
  'CSS stylesheet': 'stylesheet',
  'Dictionary': 'other',
  'Document': 'document',
  'Fetch': 'xhr',
  'Font': 'font',
  'Icon': 'other',
  'Image': 'image',
  'Internal resource': 'other',
  'Link element resource': 'other',
  'Link prefetch resource': 'speculative',
  'Manifest': 'web_manifest',
  'Mock': 'other',
  'Other resource': 'other',
  'Processing instruction': 'other',
  'Script': 'script',
  'SpeculationRule': 'speculative',
  'SVG document': 'media',
  'SVG Use element resource': 'media',
  'Text track': 'other',
  'Track': 'other',
  'User Agent CSS resource': 'stylesheet',
  'Video': 'media',
  'XML resource': 'document',
  'XMLHttpRequest': 'xhr',
  'XSL stylesheet': 'xslt'
}
/* eslint-enable quote-props */

const parser = new ArgumentParser({
  add_help: true,
  formatter_class: ArgumentDefaultsHelpFormatter,
  description: 'Check whether a URL would be blocked by given filter list rules'
})
parser.add_argument('-v', '--version', {
  action: 'version',
  version: adblockRustPackage.version
})
parser.add_argument('--url', {
  required: true,
  type: URL,
  help: 'The full URL to check against the provided filter lists.'
})
parser.add_argument('--context-url', {
  required: true,
  type: URL,
  help: 'The security context the request occurred in, as a full URL'
})
parser.add_argument('--rule-files', {
  required: true,
  type: FileType('r'),
  nargs: '*',
  help: 'One or more paths to files of filter list rules to check the ' +
        'request against'
})
parser.add_argument('--verbose', {
  default: false,
  action: 'store_true',
  help: 'Print information about what rule(s) the request matched.'
})

const requestTypeGroup = parser.add_mutually_exclusive_group(true)
requestTypeGroup.add_argument('--type', {
  help: 'The type of the request, using the types defined by ' +
        'filter list projects',
  choices: filterListRequestTypes
})
requestTypeGroup.add_argument('--chromium-type', {
  help: 'The type of the request, using the types defined by chromium',
  choices: Object.keys(chromiumRequestTypeMapping)
})

;(async () => {
  const args = parser.parse_args()

  const filterSet = new adblockRust.FilterSet(true)
  for (const aRuleFile of args.rule_files) {
    const rulesText = await text(aRuleFile)
    filterSet.addFilters(rulesText.split('\n'))
  }

  const engine = new adblockRust.Engine(filterSet, true)
  const result = engine.check(
    args.url.toString(),
    args.context_url.toString(),
    args.type || chromiumRequestTypeMapping[args.chromium_type],
    true
  )

  if (args.verbose) {
    console.log(result)
  }
  process.exit(result.matched ? 0 : 1)
})()
