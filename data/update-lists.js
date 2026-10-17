// A script to update the test lists and resources.
// Use: BRAVE_SERVICE_KEY=<key> node data/update-lists.js \
//      <brave_list_version> <defalt_privacy_filters_version> <resource_list_version>

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

// Remove readline and use command line arguments
const args = process.argv.slice(2);

if (args.length < 2) {
  console.error(
    "Usage: BRAVE_SERVICE_KEY=<key> node update-lists.js <defalt_adblock_filters_version> <defalt_privacy_filters_version> <resource_list_version>\n" +
      "The component names are: \n" +
      "Brave Default Adblock Filters\n" +
      "Brave Default Privacy Filters\n" +
      "Brave Ad Block Resources Library\n"
  );
  process.exit(1);
}

const apiKey = process.env["BRAVE_SERVICE_KEY"];
if (!apiKey) {
  console.error("Error: BRAVE_SERVICE_KEY is not set");
  process.exit(1);
}
const braveDefaultAdblockFiltersVersionNumber = args[0].replace(/\./g, "_");
const braveDefaultPrivacyFiltersVersionNumber = args[1].replace(/\./g, "_");
const resourceVersionNumber = args[2].replace(/\./g, "_");

const braveDefaultAdblockFiltersId = "iodkpdagapdfkphljnddpjlldadblomo";
const braveDefaultPrivacyFiltersId = "kihnoaefogbkmblfimmibknnmkllbhlf";
const braveResourceListId = "mfddibmblmbccpadfndgakiopmmhebop";

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
  fs.mkdtempSync("temp-list", {
    dir: rootDir,
  })
);

try {
  process.chdir(tempDir);

  execSync(
    `curl -o brave-default-adblock-filters.zip -H "BraveServiceKey: ${apiKey}" ` +
      `https://brave-core-ext.s3.brave.com/release/${braveDefaultAdblockFiltersId}/extension_${braveDefaultAdblockFiltersVersionNumber}.crx`
  );

  execSync(
    `curl -o brave-default-privacy-filters.zip -H "BraveServiceKey: ${apiKey}" ` +
      `https://brave-core-ext.s3.brave.com/release/${braveDefaultPrivacyFiltersId}/extension_${braveDefaultPrivacyFiltersVersionNumber}.crx`
  );

  execSync(
    `curl -o resources.zip -H "BraveServiceKey: ${apiKey}" ` +
      `https://brave-core-ext.s3.brave.com/release/${braveResourceListId}/extension_${resourceVersionNumber}.crx`
  );


  const takeFile = (zipFile, fileName, outputFileName) => {
    try {
      execSync(`unzip ${zipFile} -d .`);
    } catch (e) {
      // .crx is not a zip file, so we expect an error here.
      if (!fs.existsSync(fileName)) {
        throw new Error(`Failed to find ${fileName} in ${zipFile}`);
      }
    }
    fs.renameSync(fileName, path.join(rootDir, "data/brave", outputFileName));
  }

  // Merge the two default lists into one
  takeFile("brave-default-adblock-filters.zip", "list.txt", "default-1.txt");
  takeFile("brave-default-privacy-filters.zip", "list.txt", "default-2.txt");
  const defaultContent1 = fs.readFileSync(path.join(rootDir, "data/brave/default-1.txt"), { encoding: "utf-8" });
  const defaultContent2 = fs.readFileSync(path.join(rootDir, "data/brave/default-2.txt"), { encoding: "utf-8" });
  fs.writeFileSync(path.join(rootDir, "data/brave/brave-main-list.txt"), defaultContent1 + "\n" + defaultContent2);

  takeFile("resources.zip", "resources.json", "brave-resources.json");

} finally {
  fs.rmSync(tempDir, { recursive: true });
}
