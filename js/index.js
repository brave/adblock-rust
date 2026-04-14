const native = require('./index.node');

exports.FilterSet = native.FilterSet;
exports.Engine = native.Engine;
exports.validateRequest = native.validateRequest;
exports.uBlockResources = native.uBlockResources;

exports.FilterFormat = Object.freeze(native.FilterFormat());
exports.RuleTypes = Object.freeze(native.RuleTypes());
