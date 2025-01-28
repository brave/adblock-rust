const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

// Remove readline and use command line arguments
const args = process.argv.slice(2);

if (args.length < 2) {
  console.error(
    "Usage: node update-lists.js <Brave Services Key> <target version for brave list (i.e. 1.0.10268)>"
  );
  process.exit(1);
}

const apiKey = args[0];
const version = args[1];

const versionNumber = version.replace(/\./g, "_");
const extensionId = "iodkpdagapdfkphljnddpjlldadblomo";

execSync(
  "curl -o data/easylist.to/easylist/easylist.txt https://easylist.to/easylist/easylist.txt"
);
execSync(
  "curl -o data/easylist.to/easylist/easyprivacy.txt https://easylist.to/easylist/easyprivacy.txt"
);
execSync(
  "curl -o data/easylist.to/easylistgermany/easylistgermany.txt https://easylist.to/easylistgermany/easylistgermany.txt"
);

execSync(
  `curl -o extension.zip -H "BraveServiceKey: ${apiKey}" ` +
    `https://brave-core-ext.s3.brave.com/release/${extensionId}/extension_${versionNumber}.crx`
);

const tempDir = fs.mkdtempSync("temp-brave-list");
const listPath = path.join(tempDir, "list.txt");
try {
  execSync("unzip extension.zip -d " + tempDir);
} catch (e) {
  if (!fs.existsSync(listPath)) {
    console.error("Failed to find list.txt in extension.zip");
    process.exit(1);
  }
}

execSync(`mv -f ${listPath} data/brave/brave-main-list.txt`);
