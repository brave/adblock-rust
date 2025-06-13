const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

// Remove readline and use command line arguments
const args = process.argv.slice(2);

if (args.length < 2) {
  console.error(
    "Usage: node update-lists.js <Brave Services Key> <target version for brave list (i.e. 1.0.10268)>\n" +
      "The component name is 'Brave Ad Block Updater'"
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

const rootDir = path.join(__dirname, "..");
const tempDir = path.resolve(
  fs.mkdtempSync("temp-brave-list", {
    dir: rootDir,
  })
);

try {
  process.chdir(tempDir);

  execSync(
    `curl -o extension.zip -H "BraveServiceKey: ${apiKey}" ` +
      `https://brave-core-ext.s3.brave.com/release/${extensionId}/extension_${versionNumber}.crx`
  );

  const listPath = path.join(tempDir, "list.txt");
  try {
    execSync("unzip extension.zip -d .");
  } catch (e) {
    // .crx is not a zip file, so we expect an error here.
    if (!fs.existsSync(listPath)) {
      throw new Error("Failed to find list.txt in extension.zip");
    }
  }

  fs.renameSync(listPath, path.join(rootDir, "data/brave/brave-main-list.txt"));
} finally {
  fs.rmdirSync(tempDir, { recursive: true });
}
