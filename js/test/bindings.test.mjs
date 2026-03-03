/**
 * Integration tests for the adblock-rs native Node.js addon (js/index.js).
 *
 * These tests document the current behaviour of the Neon binding layer and
 * serve as a regression baseline for the NAPI-RS migration.
 *
 * Run:  node --test js/test/bindings.test.mjs
 *       (requires the native addon to be built first: npm run build)
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const { FilterSet, Engine, FilterFormat, RuleTypes, uBlockResources } =
    createRequire(import.meta.url)(join(__dirname, '..', 'index.js'));

// ---------------------------------------------------------------------------
// FilterSet.addFilters
// ---------------------------------------------------------------------------

describe('FilterSet.addFilters', () => {
    it('parses metadata comments (title, homepage, expires, redirect)', () => {
        const fs = new FilterSet();
        const meta = fs.addFilters([
            '! Title: Test List',
            '! Homepage: https://example.com',
            '! Expires: 2 days',
            '! Redirect: https://example.com/new-list.txt',
            '||ads.com^',
        ]);
        assert.equal(meta.title, 'Test List');
        assert.equal(meta.homepage, 'https://example.com');
        assert.ok(meta.expires != null);
        assert.equal(meta.redirect, 'https://example.com/new-list.txt');
    });

    it('hosts format parses IP-hostname entries', () => {
        const fs = new FilterSet();
        fs.addFilters(['127.0.0.1 ads.example.com'], { format: FilterFormat.HOSTS });
        const engine = new Engine(fs, true);
        assert.equal(
            engine.check('https://ads.example.com/', 'https://pub.com', 'script'),
            true,
        );
    });

    it('NETWORK_ONLY skips cosmetic rules', () => {
        const fs = new FilterSet();
        fs.addFilters(
            ['||example.com^', 'example.com##.ad'],
            { rule_types: RuleTypes.NETWORK_ONLY },
        );
        const engine = new Engine(fs, true);
        assert.equal(
            engine.check('https://example.com/test.js', 'https://pub.com', 'script'),
            true,
        );
        const cosmetic = engine.urlCosmeticResources('https://example.com/page');
        assert.equal(cosmetic.hide_selectors.length, 0);
    });

    it('COSMETIC_ONLY skips network rules', () => {
        const fs = new FilterSet();
        fs.addFilters(
            ['||example.com^', 'example.com##.ad'],
            { rule_types: RuleTypes.COSMETIC_ONLY },
        );
        const engine = new Engine(fs, true);
        assert.equal(
            engine.check('https://example.com/test.js', 'https://pub.com', 'script'),
            false,
        );
        const cosmetic = engine.urlCosmeticResources('https://example.com/page');
        assert.ok(cosmetic.hide_selectors.includes('.ad'));
    });
});

// ---------------------------------------------------------------------------
// FilterSet.addFilter
// ---------------------------------------------------------------------------

describe('FilterSet.addFilter', () => {
    it('returns true for a valid network filter', () => {
        const fs = new FilterSet();
        assert.equal(fs.addFilter('||example.com^'), true);
    });

    it('returns true for a valid cosmetic filter', () => {
        const fs = new FilterSet();
        assert.equal(fs.addFilter('example.com##.banner'), true);
    });

    it('returns false for an empty string', () => {
        const fs = new FilterSet();
        assert.equal(fs.addFilter(''), false);
    });

    it('returns false for a comment line', () => {
        const fs = new FilterSet();
        assert.equal(fs.addFilter('! this is a comment'), false);
    });

    it('accepts hosts format via ParseOptions', () => {
        const fs = new FilterSet();
        assert.equal(
            fs.addFilter('127.0.0.1 ads.example.com', { format: FilterFormat.HOSTS }),
            true,
        );
    });
});

// ---------------------------------------------------------------------------
// FilterSet.intoContentBlocking
// ---------------------------------------------------------------------------

describe('FilterSet.intoContentBlocking', () => {
    it('converts network filters to content blocking rules with trigger/action', () => {
        const fs = new FilterSet(true);
        fs.addFilters(['||ads.example.com^']);
        const result = fs.intoContentBlocking();
        assert.notEqual(result, undefined);
        assert.ok(result.contentBlockingRules.length > 0);
        assert.ok('trigger' in result.contentBlockingRules[0]);
        assert.ok('action' in result.contentBlockingRules[0]);
        assert.ok(result.filtersUsed.includes('||ads.example.com^'));
    });

    it('returns undefined when debug=false', () => {
        const fs = new FilterSet(false);
        fs.addFilters(['||ads.example.com^']);
        assert.equal(fs.intoContentBlocking(), undefined);
    });
});

// ---------------------------------------------------------------------------
// FilterSet clone semantics
// ---------------------------------------------------------------------------

describe('FilterSet survives Engine construction (clone semantics)', () => {
    it('FilterSet is still usable after being passed to Engine constructor', () => {
        const fs = new FilterSet();
        fs.addFilters(['||example.com^']);
        const _engine = new Engine(fs, true);
        assert.doesNotThrow(() => fs.addFilter('||another.com^'));
    });
});

// ---------------------------------------------------------------------------
// Engine.check — basic blocking
// ---------------------------------------------------------------------------

describe('Engine.check — basic blocking', () => {
    it('blocks matching requests, allows non-matching', () => {
        const fs = new FilterSet();
        fs.addFilters(['||ads.example.com^']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://ads.example.com/t.js', 'https://pub.com', 'script'), true);
        assert.equal(engine.check('https://safe.com/t.js', 'https://pub.com', 'script'), false);
    });

    it('debug=true returns BlockerResult with filter text', () => {
        const fs = new FilterSet(true);
        fs.addFilters(['||ads.example.com^']);
        const engine = new Engine(fs, true);
        const result = engine.check(
            'https://ads.example.com/t.js', 'https://pub.com', 'script', true,
        );
        assert.equal(result.matched, true);
        assert.equal(result.filter, '||ads.example.com^');
    });

    it('throws for an invalid URL', () => {
        const engine = new Engine(new FilterSet(), true);
        assert.throws(() => engine.check('not a url', 'https://publisher.com', 'script'));
    });

    it('EngineOptions object works as alternative to boolean', () => {
        const fs = new FilterSet();
        fs.addFilters(['||blocked.com^']);
        const engine = new Engine(fs, { optimize: false });
        assert.equal(
            engine.check('https://blocked.com/img.png', 'https://pub.com', 'image'),
            true,
        );
    });
});

// ---------------------------------------------------------------------------
// Engine.check — exception rules
// ---------------------------------------------------------------------------

describe('Engine.check — exception rules', () => {
    it('exception rule prevents blocking, populates exception field', () => {
        const fs = new FilterSet(true);
        fs.addFilters(['||ads.example.com^', '@@||ads.example.com^$domain=publisher.com']);
        const engine = new Engine(fs, true);
        const result = engine.check(
            'https://ads.example.com/tracker.js', 'https://publisher.com', 'script', true,
        );
        assert.equal(result.matched, false);
        assert.equal(typeof result.exception, 'string');
        assert.equal(result.exception, '@@||ads.example.com^$domain=publisher.com');
    });

    it('$important overrides exception rules', () => {
        const fs = new FilterSet(true);
        fs.addFilters(['||ads.example.com^$important', '@@||ads.example.com^']);
        const engine = new Engine(fs, true);
        const result = engine.check(
            'https://ads.example.com/t.js', 'https://pub.com', 'script', true,
        );
        assert.equal(result.matched, true);
        assert.equal(result.important, true);
        assert.equal(result.filter, '||ads.example.com^$important');
    });
});

// ---------------------------------------------------------------------------
// Engine.check — $third-party and $1p modifiers
// ---------------------------------------------------------------------------

describe('Engine.check — $third-party and $1p modifiers', () => {
    it('$third-party rule blocks 3p, allows 1p', () => {
        const fs = new FilterSet();
        fs.addFilters(['||tracker.com^$third-party']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://tracker.com/t.js', 'https://other.com', 'script'), true);
        assert.equal(engine.check('https://tracker.com/t.js', 'https://tracker.com', 'script'), false);
    });

    it('$1p rule blocks 1p, allows 3p', () => {
        const fs = new FilterSet();
        fs.addFilters(['/bad-path$1p']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://example.com/bad-path', 'https://example.com', 'script'), true);
        assert.equal(engine.check('https://example.com/bad-path', 'https://other.com', 'script'), false);
    });
});

// ---------------------------------------------------------------------------
// Engine.check — type-specific rules
// ---------------------------------------------------------------------------

describe('Engine.check — type-specific rules', () => {
    it('$script blocks only script requests', () => {
        const fs = new FilterSet();
        fs.addFilters(['||ads.example.com^$script']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://ads.example.com/t.js', 'https://pub.com', 'script'), true);
        assert.equal(engine.check('https://ads.example.com/t.png', 'https://pub.com', 'image'), false);
    });

    it('$image blocks only image requests', () => {
        const fs = new FilterSet();
        fs.addFilters(['||ads.example.com^$image']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://ads.example.com/t.png', 'https://pub.com', 'image'), true);
        assert.equal(engine.check('https://ads.example.com/t.js', 'https://pub.com', 'script'), false);
    });

    it('$stylesheet blocks only stylesheet requests', () => {
        const fs = new FilterSet();
        fs.addFilters(['||ads.example.com^$stylesheet']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://ads.example.com/s.css', 'https://pub.com', 'stylesheet'), true);
        assert.equal(engine.check('https://ads.example.com/s.js', 'https://pub.com', 'script'), false);
    });

    it('$xmlhttprequest blocks only XHR requests', () => {
        const fs = new FilterSet();
        fs.addFilters(['||ads.example.com^$xmlhttprequest']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://ads.example.com/api', 'https://pub.com', 'xmlhttprequest'), true);
        assert.equal(engine.check('https://ads.example.com/api', 'https://pub.com', 'image'), false);
    });
});

// ---------------------------------------------------------------------------
// Engine.check — $domain modifier
// ---------------------------------------------------------------------------

describe('Engine.check — $domain modifier', () => {
    it('blocks only when source matches the domain option', () => {
        const fs = new FilterSet();
        fs.addFilters(['/ads.js$domain=publisher.com']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://cdn.example.com/ads.js', 'https://publisher.com', 'script'), true);
        assert.equal(engine.check('https://cdn.example.com/ads.js', 'https://other.com', 'script'), false);
    });

    it('~domain excludes the specified domain', () => {
        const fs = new FilterSet();
        fs.addFilters(['/ads.js$domain=~safe.com']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://cdn.example.com/ads.js', 'https://safe.com', 'script'), false);
        assert.equal(engine.check('https://cdn.example.com/ads.js', 'https://other.com', 'script'), true);
    });
});

// ---------------------------------------------------------------------------
// Engine.check — $badfilter modifier
// ---------------------------------------------------------------------------

describe('Engine.check — $badfilter modifier', () => {
    it('cancels a matching blocking rule', () => {
        const fs = new FilterSet();
        fs.addFilters(['||ads.example.com^', '||ads.example.com^$badfilter']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://ads.example.com/t.js', 'https://pub.com', 'script'), false);
    });

    it('does not cancel a dissimilar rule', () => {
        const fs = new FilterSet();
        fs.addFilters(['||ads.example.com^', '||other.com^$badfilter']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://ads.example.com/t.js', 'https://pub.com', 'script'), true);
    });
});

// ---------------------------------------------------------------------------
// Engine.check — redirect rules
// ---------------------------------------------------------------------------

describe('Engine.check — redirect rules', () => {
    it('redirect field is set when $redirect rule matches and resource is loaded', () => {
        const fs = new FilterSet(true);
        fs.addFilters(['||ads.example.com^$script,redirect=noopjs']);
        const engine = new Engine(fs, true);
        engine.useResources([{
            name: 'noopjs',
            aliases: [],
            kind: { mime: 'application/javascript' },
            content: btoa('(function(){})()'),
        }]);
        const result = engine.check(
            'https://ads.example.com/t.js', 'https://pub.com', 'script', true,
        );
        assert.equal(result.matched, true);
        assert.equal(result.filter, '||ads.example.com^$script,redirect=noopjs');
        assert.ok(result.redirect.length > 0);
    });

    it('redirect is null when no redirect rule applies', () => {
        const fs = new FilterSet(true);
        fs.addFilters(['||ads.example.com^']);
        const engine = new Engine(fs, true);
        const result = engine.check(
            'https://ads.example.com/t.js', 'https://pub.com', 'script', true,
        );
        assert.equal(result.matched, true);
        assert.equal(result.filter, '||ads.example.com^');
        assert.ok(result.redirect == null);
    });
});

// ---------------------------------------------------------------------------
// Engine.check — $removeparam modifier
// ---------------------------------------------------------------------------

describe('Engine.check — $removeparam modifier', () => {
    it('strips the specified parameter, preserves others', () => {
        const fs = new FilterSet(true);
        fs.addFilters(['||example.com^$removeparam=tracking_id']);
        const engine = new Engine(fs, true);
        const result = engine.check(
            'https://example.com/page?tracking_id=abc&keep=1',
            'https://other.com', 'xmlhttprequest', true,
        );
        assert.ok(result.rewritten_url != null);
        assert.ok(!result.rewritten_url.includes('tracking_id'));
        assert.ok(result.rewritten_url.includes('keep=1'));
    });

    it('rewritten_url is null when the parameter is absent', () => {
        const fs = new FilterSet(true);
        fs.addFilters(['||example.com^$removeparam=tracking_id']);
        const engine = new Engine(fs, true);
        const result = engine.check(
            'https://example.com/page?unrelated=1',
            'https://other.com', 'xmlhttprequest', true,
        );
        assert.ok(result.rewritten_url == null);
    });
});

// ---------------------------------------------------------------------------
// Engine.check — exception rules with tags
// ---------------------------------------------------------------------------

describe('Engine.check — exception rules with tags', () => {
    it('tagged exception activates only after enableTag', () => {
        const fs = new FilterSet(true);
        fs.addFilters(['||ads.example.com^', '@@||ads.example.com^$tag=unbreak']);
        const engine = new Engine(fs, true);

        const before = engine.check(
            'https://ads.example.com/t.js', 'https://pub.com', 'script', true,
        );
        assert.equal(before.matched, true);
        assert.ok(before.exception == null);

        engine.enableTag('unbreak');
        const after = engine.check(
            'https://ads.example.com/t.js', 'https://pub.com', 'script', true,
        );
        assert.equal(after.matched, false);
        assert.equal(after.exception, '@@||ads.example.com^$tag=unbreak');
    });
});

// ---------------------------------------------------------------------------
// Engine.urlCosmeticResources
// ---------------------------------------------------------------------------

describe('Engine.urlCosmeticResources', () => {
    it('returns matching hide_selectors for the URL', () => {
        const fs = new FilterSet();
        fs.addFilters(['example.com##.ad-banner', 'example.com##.sponsored-post']);
        const engine = new Engine(fs, true);
        const result = engine.urlCosmeticResources('https://example.com/page');
        assert.ok(result.hide_selectors.includes('.ad-banner'));
        assert.ok(result.hide_selectors.includes('.sponsored-post'));
    });

    it('returns empty hide_selectors for an unmatched URL', () => {
        const fs = new FilterSet();
        fs.addFilters(['example.com##.ad-banner']);
        const engine = new Engine(fs, true);
        const result = engine.urlCosmeticResources('https://other-site.com/page');
        assert.equal(result.hide_selectors.length, 0);
    });

    it('generichide exception sets generichide=true', () => {
        const fs = new FilterSet();
        fs.addFilters(['##.generic-ad', '@@||example.com^$generichide']);
        const engine = new Engine(fs, true);
        assert.equal(engine.urlCosmeticResources('https://example.com/page').generichide, true);
        assert.equal(engine.urlCosmeticResources('https://other.com/page').generichide, false);
    });

    it('site-specific unhide (#@#) prevents selector from appearing', () => {
        const fs = new FilterSet();
        fs.addFilters(['example.com##.ad-banner', 'example.com#@#.ad-banner']);
        const engine = new Engine(fs, true);
        const result = engine.urlCosmeticResources('https://example.com/page');
        assert.ok(!result.hide_selectors.includes('.ad-banner'));
    });

    it('generic unhide (#@#) adds to exceptions list', () => {
        const fs = new FilterSet();
        fs.addFilters(['##.generic-ad', 'example.com#@#.generic-ad']);
        const engine = new Engine(fs, true);
        const result = engine.urlCosmeticResources('https://example.com/page');
        assert.ok(result.exceptions.includes('.generic-ad'));
    });

    it('negated domain (~sub.example.com) excludes subdomain', () => {
        const fs = new FilterSet();
        fs.addFilters(['example.com,~sub.example.com##.ad']);
        const engine = new Engine(fs, true);
        assert.ok(engine.urlCosmeticResources('https://example.com/page').hide_selectors.includes('.ad'));
        assert.ok(!engine.urlCosmeticResources('https://sub.example.com/page').hide_selectors.includes('.ad'));
    });

    it('procedural_actions populated for :has-text() and :remove() filters', () => {
        const fs = new FilterSet();
        fs.addFilters([
            'example.com##.items:has-text(Sponsored)',
            'example.com##.ad-banner:remove()',
        ]);
        const engine = new Engine(fs, true);
        const result = engine.urlCosmeticResources('https://example.com/page');
        assert.ok(result.procedural_actions.length >= 2);
    });

    it('scriptlet injection populates injected_script', () => {
        const fs = new FilterSet();
        fs.addFilters(['example.com##+js(noopjs)']);
        const engine = new Engine(fs, true);
        // ##+js(noopjs) looks up "noopjs.js"; scriptlets use kind: "template"
        engine.useResources([{
            name: 'noopjs.js',
            aliases: [],
            kind: 'template',
            content: btoa('(function(){})()'),
        }]);
        const result = engine.urlCosmeticResources('https://example.com/page');
        assert.ok(result.injected_script.length > 0);
    });
});

// ---------------------------------------------------------------------------
// Engine.hiddenClassIdSelectors
// ---------------------------------------------------------------------------

describe('Engine.hiddenClassIdSelectors', () => {
    it('returns selectors matching class and id names', () => {
        const fs = new FilterSet();
        fs.addFilters(['##.a-class', '###simple-id']);
        const engine = new Engine(fs, true);
        assert.ok(engine.hiddenClassIdSelectors(['a-class'], [], []).includes('.a-class'));
        assert.ok(engine.hiddenClassIdSelectors([], ['simple-id'], []).includes('#simple-id'));
    });

    it('returns empty for unknown class/id names', () => {
        const fs = new FilterSet();
        fs.addFilters(['##.a-class']);
        const engine = new Engine(fs, true);
        assert.deepEqual(engine.hiddenClassIdSelectors(['unknown'], ['unknown'], []), []);
    });

    it('exceptions array filters out results', () => {
        const fs = new FilterSet();
        fs.addFilters(['##.a-class']);
        const engine = new Engine(fs, true);
        assert.ok(engine.hiddenClassIdSelectors(['a-class'], [], []).includes('.a-class'));
        assert.ok(!engine.hiddenClassIdSelectors(['a-class'], [], ['.a-class']).includes('.a-class'));
    });
});

// ---------------------------------------------------------------------------
// Engine serialization
// ---------------------------------------------------------------------------

describe('Engine serialization', () => {
    it('roundtrip preserves blocking and exception rules', () => {
        const fs = new FilterSet();
        fs.addFilters(['||blocked.com^', '@@||exception.blocked.com^']);
        const src = new Engine(fs, true);
        const buf = src.serialize();

        const dst = new Engine(new FilterSet(), true);
        dst.deserialize(buf);
        assert.equal(dst.check('https://blocked.com/img.png', 'https://pub.com', 'image'), true);
        assert.equal(dst.check('https://safe.com/img.png', 'https://pub.com', 'image'), false);
        assert.equal(dst.check('https://exception.blocked.com/', 'https://pub.com', 'other'), false);
    });

    it('tag enablement is NOT serialized — must re-enable after deserialize', () => {
        const fs = new FilterSet();
        fs.addFilters(['adv$tag=stuff', '||blocked.com^']);
        const src = new Engine(fs, true);
        src.enableTag('stuff');
        const buf = src.serialize();

        const dst = new Engine(new FilterSet(), true);
        dst.deserialize(buf);

        // Untagged filter works immediately
        assert.equal(dst.check('https://blocked.com/t.js', 'https://pub.com', 'script'), true);
        // Tagged filter inactive until re-enabled
        assert.equal(dst.check('https://example.com/adv', 'https://example.com', 'other'), false);
        dst.enableTag('stuff');
        assert.equal(dst.check('https://example.com/adv', 'https://example.com', 'other'), true);
    });

    it('resources are NOT serialized — must reload after deserialize', () => {
        const fs = new FilterSet(true);
        fs.addFilters(['||ads.example.com^$script,redirect=noopjs']);
        const resource = {
            name: 'noopjs',
            aliases: [],
            kind: { mime: 'application/javascript' },
            content: btoa('(function(){})()'),
        };
        const src = new Engine(fs, true);
        src.useResources([resource]);
        const buf = src.serialize();

        const dst = new Engine(new FilterSet(), true);
        dst.deserialize(buf);

        // Without reloading: redirect absent
        const without = dst.check(
            'https://ads.example.com/t.js', 'https://pub.com', 'script', true,
        );
        assert.equal(without.matched, true);
        assert.ok(without.redirect == null);

        // After reloading: redirect works
        dst.useResources([resource]);
        const withRes = dst.check(
            'https://ads.example.com/t.js', 'https://pub.com', 'script', true,
        );
        assert.ok(withRes.redirect.length > 0);
    });
});

// ---------------------------------------------------------------------------
// Engine tags
// ---------------------------------------------------------------------------

describe('Engine tags', () => {
    it('tagged filter is inactive before enableTag, active after', () => {
        const fs = new FilterSet();
        fs.addFilters(['adv$tag=stuff']);
        const engine = new Engine(fs, true);
        assert.equal(engine.check('https://example.com/adv', 'https://example.com', 'other'), false);
        engine.enableTag('stuff');
        assert.equal(engine.check('https://example.com/adv', 'https://example.com', 'other'), true);
        assert.equal(engine.tagExists('stuff'), true);
    });

    it('clearTags deactivates all enabled tags', () => {
        const fs = new FilterSet();
        fs.addFilters(['adv$tag=stuff', '||brianbondy.com/$tag=brian']);
        const engine = new Engine(fs, true);
        engine.enableTag('stuff');
        engine.enableTag('brian');
        assert.equal(engine.check('https://example.com/adv', 'https://example.com', 'other'), true);

        engine.clearTags();
        assert.equal(engine.tagExists('stuff'), false);
        assert.equal(engine.tagExists('brian'), false);
        assert.equal(engine.check('https://example.com/adv', 'https://example.com', 'other'), false);
    });
});

// ---------------------------------------------------------------------------
// uBlockResources
// ---------------------------------------------------------------------------

describe('uBlockResources', () => {
    const dataDir = join(__dirname, '..', '..', 'data', 'test', 'fake-uBO-files');

    it('returns resources that work with engine.useResources', () => {
        const resources = uBlockResources(
            join(dataDir, 'web_accessible_resources'),
            join(dataDir, 'redirect-resources.js'),
        );
        assert.ok(resources.length > 0);
        const engine = new Engine(new FilterSet(), true);
        assert.doesNotThrow(() => engine.useResources(resources));
    });

    it('accepts optional scriptlets path', () => {
        const resources = uBlockResources(
            join(dataDir, 'web_accessible_resources'),
            join(dataDir, 'redirect-resources.js'),
            join(dataDir, 'scriptlets.js'),
        );
        assert.ok(resources.length > 0);
    });
});
