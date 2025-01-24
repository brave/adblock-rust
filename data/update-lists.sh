#!/bin/sh
curl -o easylist.to/easylist/easylist.txt https://easylist.to/easylist/easylist.txt
curl -o easylist.to/easylist/easyprivacy.txt https://easylist.to/easylist/easyprivacy.txt
curl -o easylist.to/easylistgermany/easylistgermany.txt https://easylist.to/easylistgermany/easylistgermany.txt

echo "You need to provide service-key-from-1P and target version to update brave-main-list.txt"
echo -n "Enter service-key-from-1P: "
read API_KEY
echo -n "Enter target version (i.e 1.0.10268): "
read VERSION
VERSION_NUMBER=${VERSION//./_}
EXTENSION_ID="iodkpdagapdfkphljnddpjlldadblomo"
curl -o extension.zip -H "BraveServiceKey: $API_KEY" https://brave-core-ext.s3.brave.com/release/${EXTENSION_ID}/extension_${VERSION_NUMBER}.crx
unzip extension.zip list.txt
mv -f list.txt brave/brave-main-list.txt
