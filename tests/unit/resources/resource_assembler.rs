#[cfg(test)]
mod tests {
    use super::super::*;

    #[test]
    fn test_war_resource_assembly() {
        let web_accessible_resource_dir =
            Path::new("data/test/fake-uBO-files/web_accessible_resources");
        let redirect_resources_path = Path::new("data/test/fake-uBO-files/redirect-resources.js");
        let resources =
            assemble_web_accessible_resources(web_accessible_resource_dir, redirect_resources_path)
                .expect("assemble resources");

        let expected_resource_names = vec![
            "1x1.gif",
            "2x2.png",
            "3x2.png",
            "32x32.png",
            "addthis_widget.js",
            "amazon_ads.js",
            "amazon_apstag.js",
            "ampproject_v0.js",
            "chartbeat.js",
            //"click-to-load.html" is ignored because it has a params field.
            "doubleclick_instream_ad_status.js",
            "empty",
            "fingerprint2.js",
            "fingerprint3.js",
            "google-analytics_analytics.js",
            "google-analytics_cx_api.js",
            "google-analytics_ga.js",
            "google-analytics_inpage_linkid.js",
            "google-ima.js",
            "googlesyndication_adsbygoogle.js",
            "googletagservices_gpt.js",
            "hd-main.js",
            "ligatus_angular-tag.js",
            "mxpnl_mixpanel.js",
            "monkeybroker.js",
            "noeval.js",
            "noeval-silent.js",
            "nobab.js",
            "nobab2.js",
            "nofab.js",
            "noop-0.1s.mp3",
            "noop-0.5s.mp3",
            "noop-1s.mp4",
            "noop.html",
            "noop.js",
            "noop.txt",
            "noop-vmap1.0.xml",
            "outbrain-widget.js",
            "popads.js",
            "popads-dummy.js",
            "prebid-ads.js",
            "scorecardresearch_beacon.js",
            "window.open-defuser.js",
        ];

        for name in expected_resource_names {
            dbg!(&name);
            assert!(
                resources.iter().any(|resource| {
                    if let ResourceType::Mime(_) = resource.kind {
                        resource.name == name
                    } else {
                        false
                    }
                }),
                "{name:?}"
            );
        }

        let serialized = serde_json::to_string(&resources).expect("serialize resources");

        let reserialized: Vec<Resource> =
            serde_json::from_str(&serialized).expect("deserialize resources");

        assert_eq!(reserialized[0].name, "1x1.gif");
        assert_eq!(reserialized[0].aliases, vec!["1x1-transparent.gif"]);
        assert_eq!(reserialized[0].kind, ResourceType::Mime(MimeType::ImageGif));

        assert_eq!(reserialized[34].name, "noop.js");
        assert_eq!(
            reserialized[34].aliases,
            vec!["noopjs", "abp-resource:blank-js"]
        );
        assert_eq!(
            reserialized[34].kind,
            ResourceType::Mime(MimeType::ApplicationJavascript)
        );
        let noopjs_contents = std::fs::read_to_string(Path::new(
            "data/test/fake-uBO-files/web_accessible_resources/noop.js",
        ))
        .unwrap()
        .replace('\r', "");
        assert_eq!(
            std::str::from_utf8(
                &BASE64_STANDARD
                    .decode(&reserialized[34].content)
                    .expect("decode base64 content")
            )
            .expect("convert to utf8 string"),
            noopjs_contents,
        );
    }

    #[test]
    fn test_scriptlet_resource_assembly2() {
        let scriptlets_path = Path::new("data/test/fake-uBO-files/scriptlets2.js");
        #[allow(deprecated)]
        let resources = assemble_scriptlet_resources(scriptlets_path).expect("assemble scriptlets");

        let expected_resource_names = vec![
            "abort-current-inline-script.js",
            "abort-on-property-read.js",
            "abort-on-property-write.js",
            "abort-on-stack-trace.js",
            "addEventListener-defuser.js",
            "addEventListener-logger.js",
            "json-prune.js",
            "nano-setInterval-booster.js",
            "nano-setTimeout-booster.js",
            "noeval-if.js",
            "no-fetch-if.js",
            "no-floc.js",
            "remove-attr.js",
            "remove-class.js",
            "no-requestAnimationFrame-if.js",
            "set-constant.js",
            "no-setInterval-if.js",
            "no-setTimeout-if.js",
            "webrtc-if.js",
            "window.name-defuser",
            "overlay-buster.js",
            "alert-buster.js",
            "gpt-defuser.js",
            "nowebrtc.js",
            "golem.de.js",
            "upmanager-defuser.js",
            "smartadserver.com.js",
            "adfly-defuser.js",
            "disable-newtab-links.js",
            "damoh-defuser.js",
            "twitch-videoad.js",
            "fingerprint2.js",
            "cookie-remover.js",
        ];

        for name in expected_resource_names {
            assert!(
                resources.iter().any(|resource| {
                    match resource.kind {
                        ResourceType::Template
                        | ResourceType::Mime(MimeType::ApplicationJavascript) => {
                            resource.name == name
                        }
                        _ => false,
                    }
                }),
                "failed to find {name}"
            );
        }

        let serialized = serde_json::to_string(&resources).expect("serialize resources");

        let reserialized: Vec<Resource> =
            serde_json::from_str(&serialized).expect("deserialize resources");

        assert_eq!(reserialized[0].name, "abort-current-inline-script.js");
        assert_eq!(reserialized[0].aliases, vec!["acis.js"]);
        assert_eq!(reserialized[0].kind, ResourceType::Template);

        assert_eq!(reserialized[17].name, "no-setTimeout-if.js");
        assert_eq!(
            reserialized[17].aliases,
            vec!["nostif.js", "setTimeout-defuser.js"]
        );
        assert_eq!(reserialized[17].kind, ResourceType::Template);

        assert_eq!(reserialized[20].name, "overlay-buster.js");
        assert_eq!(reserialized[20].aliases, Vec::<String>::new());
        assert_eq!(
            reserialized[20].kind,
            ResourceType::Mime(MimeType::ApplicationJavascript)
        );
        assert_eq!(
            std::str::from_utf8(
                &BASE64_STANDARD.decode(&reserialized[20].content).expect("decode base64 content")
            ).expect("convert to utf8 string"),
            "(function() {\nif ( window !== window.top ) {\nreturn;\n}\nvar tstart;\nvar ttl = 30000;\nvar delay = 0;\nvar delayStep = 50;\nvar buster = function() {\nvar docEl = document.documentElement,\nbodyEl = document.body,\nvw = Math.min(docEl.clientWidth, window.innerWidth),\nvh = Math.min(docEl.clientHeight, window.innerHeight),\ntol = Math.min(vw, vh) * 0.05,\nel = document.elementFromPoint(vw/2, vh/2),\nstyle, rect;\nfor (;;) {\nif ( el === null || el.parentNode === null || el === bodyEl ) {\nbreak;\n}\nstyle = window.getComputedStyle(el);\nif ( parseInt(style.zIndex, 10) >= 1000 || style.position === 'fixed' ) {\nrect = el.getBoundingClientRect();\nif ( rect.left <= tol && rect.top <= tol && (vw - rect.right) <= tol && (vh - rect.bottom) < tol ) {\nel.parentNode.removeChild(el);\ntstart = Date.now();\nel = document.elementFromPoint(vw/2, vh/2);\nbodyEl.style.setProperty('overflow', 'auto', 'important');\ndocEl.style.setProperty('overflow', 'auto', 'important');\ncontinue;\n}\n}\nel = el.parentNode;\n}\nif ( (Date.now() - tstart) < ttl ) {\ndelay = Math.min(delay + delayStep, 1000);\nsetTimeout(buster, delay);\n}\n};\nvar domReady = function(ev) {\nif ( ev ) {\ndocument.removeEventListener(ev.type, domReady);\n}\ntstart = Date.now();\nsetTimeout(buster, delay);\n};\nif ( document.readyState === 'loading' ) {\ndocument.addEventListener('DOMContentLoaded', domReady);\n} else {\ndomReady();\n}\n})();\n",
        );

        assert_eq!(reserialized[6].name, "json-prune.js");
        assert_eq!(reserialized[6].aliases, Vec::<String>::new());
        assert_eq!(reserialized[6].kind, ResourceType::Template);
        assert_eq!(
            std::str::from_utf8(
                &BASE64_STANDARD.decode(&reserialized[6].content).expect("decode base64 content")
            ).expect("convert to utf8 string"),
            "(function() {\nconst rawPrunePaths = '{{1}}';\nconst rawNeedlePaths = '{{2}}';\nconst prunePaths = rawPrunePaths !== '{{1}}' && rawPrunePaths !== ''\n? rawPrunePaths.split(/ +/)\n: [];\nlet needlePaths;\nlet log, reLogNeedle;\nif ( prunePaths.length !== 0 ) {\nneedlePaths = prunePaths.length !== 0 &&\nrawNeedlePaths !== '{{2}}' && rawNeedlePaths !== ''\n? rawNeedlePaths.split(/ +/)\n: [];\n} else {\nlog = console.log.bind(console);\nlet needle;\nif ( rawNeedlePaths === '' || rawNeedlePaths === '{{2}}' ) {\nneedle = '.?';\n} else if ( rawNeedlePaths.charAt(0) === '/' && rawNeedlePaths.slice(-1) === '/' ) {\nneedle = rawNeedlePaths.slice(1, -1);\n} else {\nneedle = rawNeedlePaths.replace(/[.*+?^${}()|[\\]\\\\]/g, '\\\\$&');\n}\nreLogNeedle = new RegExp(needle);\n}\nconst findOwner = function(root, path, prune = false) {\nlet owner = root;\nlet chain = path;\nfor (;;) {\nif ( typeof owner !== 'object' || owner === null  ) {\nreturn false;\n}\nconst pos = chain.indexOf('.');\nif ( pos === -1 ) {\nif ( prune === false ) {\nreturn owner.hasOwnProperty(chain);\n}\nif ( chain === '*' ) {\nfor ( const key in owner ) {\nif ( owner.hasOwnProperty(key) === false ) { continue; }\ndelete owner[key];\n}\n} else if ( owner.hasOwnProperty(chain) ) {\ndelete owner[chain];\n}\nreturn true;\n}\nconst prop = chain.slice(0, pos);\nif (\nprop === '[]' && Array.isArray(owner) ||\nprop === '*' && owner instanceof Object\n) {\nconst next = chain.slice(pos + 1);\nlet found = false;\nfor ( const key of Object.keys(owner) ) {\nfound = findOwner(owner[key], next, prune) || found;\n}\nreturn found;\n}\nif ( owner.hasOwnProperty(prop) === false ) { return false; }\nowner = owner[prop];\nchain = chain.slice(pos + 1);\n}\n};\nconst mustProcess = function(root) {\nfor ( const needlePath of needlePaths ) {\nif ( findOwner(root, needlePath) === false ) {\nreturn false;\n}\n}\nreturn true;\n};\nconst pruner = function(o) {\nif ( log !== undefined ) {\nconst json = JSON.stringify(o, null, 2);\nif ( reLogNeedle.test(json) ) {\nlog('uBO:', location.hostname, json);\n}\nreturn o;\n}\nif ( mustProcess(o) === false ) { return o; }\nfor ( const path of prunePaths ) {\nfindOwner(o, path, true);\n}\nreturn o;\n};\nJSON.parse = new Proxy(JSON.parse, {\napply: function() {\nreturn pruner(Reflect.apply(...arguments));\n},\n});\nResponse.prototype.json = new Proxy(Response.prototype.json, {\napply: function() {\nreturn Reflect.apply(...arguments).then(o => pruner(o));\n},\n});\n})();\n",
        );
    }

    #[test]
    fn test_scriptlet_resource_assembly() {
        let scriptlets_path = Path::new("data/test/fake-uBO-files/scriptlets.js");
        #[allow(deprecated)]
        let resources = assemble_scriptlet_resources(scriptlets_path).expect("assemble scriptlets");

        let expected_resource_names = vec![
            "abort-current-inline-script.js",
            "abort-on-property-read.js",
            "abort-on-property-write.js",
            "addEventListener-defuser.js",
            "addEventListener-logger.js",
            "json-prune.js",
            "nano-setInterval-booster.js",
            "nano-setTimeout-booster.js",
            "noeval-if.js",
            "remove-attr.js",
            "requestAnimationFrame-if.js",
            "set-constant.js",
            "setInterval-defuser.js",
            "no-setInterval-if.js",
            "setTimeout-defuser.js",
            "no-setTimeout-if.js",
            "webrtc-if.js",
            "window.name-defuser",
            "overlay-buster.js",
            "alert-buster.js",
            "gpt-defuser.js",
            "nowebrtc.js",
            "golem.de.js",
            "upmanager-defuser.js",
            "smartadserver.com.js",
            "adfly-defuser.js",
            "disable-newtab-links.js",
            "damoh-defuser.js",
            "twitch-videoad.js",
            "fingerprint2.js",
            "cookie-remover.js",
        ];

        for name in expected_resource_names {
            assert!(
                resources.iter().any(|resource| {
                    match resource.kind {
                        ResourceType::Template
                        | ResourceType::Mime(MimeType::ApplicationJavascript) => {
                            resource.name == name
                        }
                        _ => false,
                    }
                }),
                "failed to find {name}"
            );
        }

        let serialized = serde_json::to_string(&resources).expect("serialize resources");

        let reserialized: Vec<Resource> =
            serde_json::from_str(&serialized).expect("deserialize resources");

        assert_eq!(reserialized[0].name, "abort-current-inline-script.js");
        assert_eq!(reserialized[0].aliases, vec!["acis.js"]);
        assert_eq!(reserialized[0].kind, ResourceType::Template);

        assert_eq!(reserialized[18].name, "overlay-buster.js");
        assert_eq!(reserialized[18].aliases, Vec::<String>::new());
        assert_eq!(
            reserialized[18].kind,
            ResourceType::Mime(MimeType::ApplicationJavascript)
        );
        assert_eq!(
            std::str::from_utf8(
                &BASE64_STANDARD.decode(&reserialized[18].content).expect("decode base64 content")
            ).expect("convert to utf8 string"),
            "(function() {\nif ( window !== window.top ) {\nreturn;\n}\nvar tstart;\nvar ttl = 30000;\nvar delay = 0;\nvar delayStep = 50;\nvar buster = function() {\nvar docEl = document.documentElement,\nbodyEl = document.body,\nvw = Math.min(docEl.clientWidth, window.innerWidth),\nvh = Math.min(docEl.clientHeight, window.innerHeight),\ntol = Math.min(vw, vh) * 0.05,\nel = document.elementFromPoint(vw/2, vh/2),\nstyle, rect;\nfor (;;) {\nif ( el === null || el.parentNode === null || el === bodyEl ) {\nbreak;\n}\nstyle = window.getComputedStyle(el);\nif ( parseInt(style.zIndex, 10) >= 1000 || style.position === 'fixed' ) {\nrect = el.getBoundingClientRect();\nif ( rect.left <= tol && rect.top <= tol && (vw - rect.right) <= tol && (vh - rect.bottom) < tol ) {\nel.parentNode.removeChild(el);\ntstart = Date.now();\nel = document.elementFromPoint(vw/2, vh/2);\nbodyEl.style.setProperty('overflow', 'auto', 'important');\ndocEl.style.setProperty('overflow', 'auto', 'important');\ncontinue;\n}\n}\nel = el.parentNode;\n}\nif ( (Date.now() - tstart) < ttl ) {\ndelay = Math.min(delay + delayStep, 1000);\nsetTimeout(buster, delay);\n}\n};\nvar domReady = function(ev) {\nif ( ev ) {\ndocument.removeEventListener(ev.type, domReady);\n}\ntstart = Date.now();\nsetTimeout(buster, delay);\n};\nif ( document.readyState === 'loading' ) {\ndocument.addEventListener('DOMContentLoaded', domReady);\n} else {\ndomReady();\n}\n})();\n",
        );
    }
}
