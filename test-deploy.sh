set -e

docker build . -t "memothelemo/eden:dev" \
    --build-arg COMMIT_HASH=$(git rev-parse HEAD) \
    --build-arg COMMIT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
