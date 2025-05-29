
echo "export const binaryName = '$1';" > client/src/binaryName.ts

cp -r "../../binaries/$1" ./media/

npm run vscode:prepublish && vsce package -o "$2.vsix" --target $3

rm -rf "./media/$1"
mv "$2.vsix" ../../extensions
