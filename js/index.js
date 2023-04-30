const blocker = require('./index.node');

// Exposes the specified native method names on `className`
function forwardClassMethods(className, methods) {
    for (const method of methods) {
        className.prototype[method] = function(...args) {
            const blocker_method = blocker[className.name + '_' + method];
            return blocker_method(this.boxed, ...args);
        }
    }
}

class FilterSet {
    constructor(...args) {
        this.boxed = blocker.FilterSet_constructor(...args);
    }
}
forwardClassMethods(FilterSet, ['addFilters', 'addFilter', 'intoContentBlocking']);

class Engine {
    constructor(filter_set, ...args) {
        this.boxed = blocker.Engine_constructor(filter_set.boxed, ...args);
    }
}
forwardClassMethods(Engine, ['check', 'urlCosmeticResources', 'hiddenClassIdSelectors', 'serializeRaw', 'serializeCompressed', 'deserialize', 'enableTag', 'useResources', 'tagExists', 'clearTags', 'addResource', 'getResource']);

exports.FilterFormat = blocker.FilterFormat;
exports.FilterSet = FilterSet;
exports.RuleTypes = blocker.RuleTypes;
exports.Engine = Engine;
exports.uBlockResources = blocker.uBlockResources;
