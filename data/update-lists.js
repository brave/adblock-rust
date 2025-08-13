// A script to update the test lists and resources.
// Use: BRAVE_SERVICE_KEY=<key> node data/update-lists.js <brave_list_version> <resource_list_version>

const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

// Remove readline and use command line arguments
const args = process.argv.slice(2);

if (args.length < 2) {
  console.error(
    "Usage: BRAVE_SERVICE_KEY=<key> node update-lists.js <brave_list_version> <resource_list_version>\n" +
      "The component names are 'Brave Default Adblock Filters' and 'Brave Ad Block Resources Library'"
  );
  process.exit(1);
}

const apiKey = process.env["BRAVE_SERVICE_KEY"];
if (!apiKey) {
  console.error("Error: BRAVE_SERVICE_KEY is not set");
  process.exit(1);
}
const braveVersionNumber = args[0].replace(/\./g, "_");
const resourceVersionNumber = args[1].replace(/\./g, "_");

const braveMainListId = "iodkpdagapdfkphljnddpjlldadblomo";
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
    `curl -o main_list.zip -H "BraveServiceKey: ${apiKey}" ` +
      `https://brave-core-ext.s3.brave.com/release/${braveMainListId}/extension_${braveVersionNumber}.crx`
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

  takeFile("main_list.zip", "list.txt", "brave-main-list.txt");
  takeFile("resources.zip", "resources.json", "brave-resources.json");

} finally {
  fs.rmSync(tempDir, { recursive: true });
}
