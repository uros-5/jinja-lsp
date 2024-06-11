
# act -j show -P macos-latest=sickcodes/docker-osx -P windows-latest=dockurr/windows
act --env-file .env -W .github/workflows/CI-nodejs.yaml -P macos-latest=sickcodes/docker-osx -P windows-latest=dockurr/windows
