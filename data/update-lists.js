const { execSync } = require("child_process");
const readline = require("readline");
const fs = require("fs");

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
});

execSync(
  "curl -o data/easylist.to/easylist/easylist.txt https://easylist.to/easylist/easylist.txt"
);
execSync(
  "curl -o data/easylist.to/easylist/easyprivacy.txt https://easylist.to/easylist/easyprivacy.txt"
);
execSync(
  "curl -o data/easylist.to/easylistgermany/easylistgermany.txt https://easylist.to/easylistgermany/easylistgermany.txt"
);

(async () => {
  console.log(
    "You need to provide Brave Services Key and target version to update brave-main-list.txt"
  );
  const apiKey = await new Promise((resolve) => {
    rl.question("Enter Brave Services Key: ", resolve);
  });

  const version = await new Promise((resolve) => {
    rl.question("Enter target version (i.e 1.0.10268): ", resolve);
  });

  const versionNumber = version.replace(/\./g, "_");
  const extensionId = "iodkpdagapdfkphljnddpjlldadblomo";

  execSync(
    `curl -o extension.zip -H "BraveServiceKey: ${apiKey}" ` +
      `https://brave-core-ext.s3.brave.com/release/${extensionId}/extension_${versionNumber}.crx`
  );

  try {
    execSync("unzip extension.zip list.txt");
  } catch (e) {
    if (!fs.existsSync("list.txt")) {
      console.error("Failed to find list.txt in extension.zip");
      process.exit(1);
    }
  }

  execSync("mv -f list.txt data/brave/brave-main-list.txt");

  fs.unlinkSync("extension.zip");
  rl.close();
})();
