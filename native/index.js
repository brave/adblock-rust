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

class Engine {
    constructor(filter_set, ...args) {
        this.boxed = blocker.Engine_constructor(filter_set.boxed, ...args);
    }

    check(...args) {
        return blocker.Engine_check(this.boxed, ...args);
    }

    serializeRaw(...args) {
        return blocker.Engine_serializeRaw(this.boxed, ...args);
    }

    serializeCompressed(...args) {
        return blocker.Engine_serializeCompressed(this.boxed, ...args);
    }

    deserialize(...args) {
        return blocker.Engine_deserialize(this.boxed, ...args);
    }

    enableTag(...args) {
        return blocker.Engine_enableTag(this.boxed, ...args);
    }

    useResources(...args) {
        return blocker.Engine_useResources(this.boxed, ...args);
    }

    tagExists(...args) {
        return blocker.Engine_tagExists(this.boxed, ...args);
    }

    clearTags(...args) {
        return blocker.Engine_clearTags(this.boxed, ...args);
    }

    addResource(...args) {
        return blocker.Engine_addResource(this.boxed, ...args);
    }

    getResource(...args) {
        return blocker.Engine_getResource(this.boxed, ...args);
    }
}

exports.FilterFormat = blocker.FilterFormat;
exports.FilterSet = FilterSet;
exports.RuleTypes = blocker.RuleTypes;
exports.Engine = Engine;
exports.uBlockResources = blocker.uBlockResources;
