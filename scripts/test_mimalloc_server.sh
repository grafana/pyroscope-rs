#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
artifacts="${PYROSCOPE_TEST_ARTIFACT_DIR:-$root/target/mimalloc-server-e2e}"
mkdir -p "$artifacts"
image="${PYROSCOPE_TEST_IMAGE:-grafana/pyroscope:2.4.0@sha256:b7e62699a27463aa3533efe3f88c320da594da495731fd6eabca6f44f92cdd7b}"
write_path="${PYROSCOPE_TEST_WRITE_PATH:-ingester}"
case "$write_path" in
    ingester) read_args=(-enable-query-backend=false) ;;
    segment-writer) read_args=(-enable-query-backend-from=1970-01-01T00:00:00Z) ;;
    *) echo 'PYROSCOPE_TEST_WRITE_PATH must be ingester or segment-writer' >&2; exit 1 ;;
esac
container=""
cleanup() {
    local status=$?
    if [[ -n "$container" ]]; then
        docker logs "$container" > "$artifacts/server.log" 2>&1 || true
        docker rm --force "$container" > /dev/null 2>&1 || true
    fi
    return "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

printf 'image=%s\nwrite_path=%s\nsource_revision=%s\n' "$image" "$write_path" "$(git rev-parse HEAD)" > "$artifacts/server.env"
if [[ -n "$(git status --porcelain)" ]]; then
    echo 'worktree_dirty=true' >> "$artifacts/server.env"
else
    echo 'worktree_dirty=false' >> "$artifacts/server.env"
fi
container="$(docker run --detach --name "pyroscope-mimalloc-e2e-$$-$RANDOM" \
    --cpus 2 --memory 1g --workdir /data --tmpfs /data:rw,size=256m,mode=1777 \
    -p 127.0.0.1::4040 "$image" \
    -target=all -write-path="$write_path" "${read_args[@]}" -log.level=debug -usage-stats.enabled=false -self-profiling.disable-push=true \
    -ingester.min-ready-duration=0s -segment-writer.min-ready-duration=0s -metastore.min-ready-duration=0s \
    -pyroscopedb.data-path=/data/v1 -storage.filesystem.dir=/data/shared)"
address="$(docker port "$container" 4040/tcp)"
url="http://$address"
ready=false
for _ in {1..90}; do
    if curl --fail --silent --max-time 1 "$url/ready" > /dev/null; then
        ready=true
        break
    fi
    if [[ "$(docker inspect --format '{{.State.Running}}' "$container" 2>/dev/null)" != true ]]; then
        echo 'Pyroscope exited before readiness' >&2
        exit 1
    fi
    sleep 1
done
if [[ "$ready" != true ]]; then
    echo 'Pyroscope did not become ready within 90 seconds' >&2
    exit 1
fi
PYROSCOPE_TEST_URL="$url" PYROSCOPE_TEST_ARTIFACT_DIR="$artifacts" \
    cargo test --locked --features backend-mimalloc --test mimalloc_server -- --ignored --nocapture \
    2>&1 | tee "$artifacts/test.log"
