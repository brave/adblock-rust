const blocker = require('./index.node');

class FilterSet {
    constructor(...args) {
        this.boxed = blocker.FilterSet_constructor(...args);
    }

    addFilters(...args) {
        return blocker.FilterSet_addFilters(this.boxed, ...args);
    }

    addFilter(...args) {
        return blocker.FilterSet_addFilter(this.boxed, ...args);
    }

    intoContentBlocking(...args) {
        return blocker.FilterSet_intoContentBlocking(this.boxed, ...args);
    }
}

exports.FilterFormat = blocker.FilterFormat;
exports.FilterSet = FilterSet;
exports.RuleTypes = blocker.RuleTypes;
exports.Engine = blocker.Engine;
exports.uBlockResources = blocker.uBlockResources;
