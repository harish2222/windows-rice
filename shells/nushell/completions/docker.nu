# ============================================================================
# Author -: HKDEVS
# Project -: Window-Rice
# Title -: shells\nushell\completions / docker
# ============================================================================
# Docker custom completions for Nushell

# ── Container names ──
export def "nu-complete docker containers" [] {
    ^docker ps -a --format '{{.Names}}' 2>/dev/null | lines | each { |c| { value: $c description: "container" } }
}

# ── Running container names ──
export def "nu-complete docker containers running" [] {
    ^docker ps --format '{{.Names}}' 2>/dev/null | lines | each { |c| { value: $c description: "running" } }
}

# ── Image names ──
export def "nu-complete docker images" [] {
    ^docker images --format '{{.Repository}}:{{.Tag}}' 2>/dev/null | lines | each { |i| { value: $i description: "image" } }
}

# ── Image repository names ──
export def "nu-complete docker image repos" [] {
    ^docker images --format '{{.Repository}}' 2>/dev/null | lines | uniq | each { |r| { value: $r description: "repository" } }
}

# ── Volume names ──
export def "nu-complete docker volumes" [] {
    ^docker volume ls --format '{{.Name}}' 2>/dev/null | lines | each { |v| { value: $v description: "volume" } }
}

# ── Network names ──
export def "nu-complete docker networks" [] {
    ^docker network ls --format '{{.Name}}' 2>/dev/null | lines | each { |n| { value: $n description: "network" } }
}

# ── Docker Compose service names ──
export def "nu-complete docker compose services" [] {
    if ("docker-compose.yml" | path exists) or ("docker-compose.yaml" | path exists) {
        ^docker compose config --services 2>/dev/null | lines | each { |s| { value: $s description: "service" } }
    } else { [] }
}

# ── Docker subcommands ──
export def "nu-complete docker subcommands" [] {
    [
        { value: "attach" description: "Attach local standard input to a running container" }
        { value: "build" description: "Build an image from a Dockerfile" }
        { value: "commit" description: "Create a new image from a container's changes" }
        { value: "config" description: "Manage Docker configs" }
        { value: "container" description: "Manage containers" }
        { value: "context" description: "Manage contexts" }
        { value: "cp" description: "Copy files/folders between a container and the local filesystem" }
        { value: "create" description: "Create a new container" }
        { value: "diff" description: "Inspect changes to files or directories on a container's filesystem" }
        { value: "events" description: "Get real time events from the server" }
        { value: "exec" description: "Run a command in a running container" }
        { value: "export" description: "Export a container's filesystem as a tar archive" }
        { value: "history" description: "Show the history of an image" }
        { value: "images" description: "List images" }
        { value: "import" description: "Import the contents from a tarball to create a filesystem image" }
        { value: "info" description: "Display system-wide information" }
        { value: "inspect" description: "Return low-level information on Docker objects" }
        { value: "kill" description: "Kill one or more running containers" }
        { value: "load" description: "Load an image from a tar archive or STDIN" }
        { value: "login" description: "Log in to a Docker registry" }
        { value: "logout" description: "Log out from a Docker registry" }
        { value: "logs" description: "Fetch the logs of a container" }
        { value: "network" description: "Manage networks" }
        { value: "pause" description: "Pause all processes within one or more containers" }
        { value: "port" description: "List port mappings for a container" }
        { value: "ps" description: "List containers" }
        { value: "pull" description: "Pull an image or a repository from a registry" }
        { value: "push" description: "Push an image or a repository to a registry" }
        { value: "rename" description: "Rename a container" }
        { value: "restart" description: "Restart one or more containers" }
        { value: "rm" description: "Remove one or more containers" }
        { value: "rmi" description: "Remove one or more images" }
        { value: "run" description: "Run a command in a new container" }
        { value: "save" description: "Save one or more images to a tar archive" }
        { value: "search" description: "Search the Docker Hub for images" }
        { value: "start" description: "Start one or more stopped containers" }
        { value: "stats" description: "Display a live stream of container resource usage statistics" }
        { value: "stop" description: "Stop one or more running containers" }
        { value: "system" description: "Manage Docker" }
        { value: "tag" description: "Create a tag TARGET_IMAGE that refers to SOURCE_IMAGE" }
        { value: "top" description: "Display the running processes of a container" }
        { value: "unpause" description: "Unpause all processes within one or more containers" }
        { value: "update" description: "Update configuration of one or more containers" }
        { value: "version" description: "Show the Docker version information" }
        { value: "volume" description: "Manage volumes" }
        { value: "wait" description: "Block until one or more containers stop, then print their exit codes" }
    ]
}

# ── Docker Compose subcommands ──
export def "nu-complete docker compose subcommands" [] {
    [
        { value: "attach" description: "Attach to a running container" }
        { value: "build" description: "Build or rebuild services" }
        { value: "config" description: "View, edit, and validate the Compose configuration" }
        { value: "create" description: "Create containers for a service" }
        { value: "down" description: "Stop and remove containers, networks" }
        { value: "events" description: "Receive real time events from containers" }
        { value: "exec" description: "Execute a command in a running container" }
        { value: "images" description: "List images used by the created containers" }
        { value: "kill" description: "Force stop running containers" }
        { value: "logs" description: "View container output" }
        { value: "pause" description: "Pause services" }
        { value: "port" description: "List port mappings" }
        { value: "ps" description: "List running containers" }
        { value: "pull" description: "Pull service images" }
        { value: "push" description: "Push service images" }
        { value: "restart" description: "Restart services" }
        { value: "rm" description: "Remove stopped containers" }
        { value: "run" description: "Run a one-off command" }
        { value: "scale" description: "Scale services" }
        { value: "start" description: "Start services" }
        { value: "stats" description: "Display resource usage statistics" }
        { value: "stop" description: "Stop services" }
        { value: "top" description: "Display the running processes" }
        { value: "unpause" description: "Unpause services" }
        { value: "up" description: "Create and start containers" }
        { value: "version" description: "Show the Docker Compose version" }
    ]
}

# ── Extern definitions ──
export extern "docker" [
    subcommand: string@"nu-complete docker subcommands"
]

export extern "docker run" [
    image: string@"nu-complete docker images"
    ...commands: string
    --name: string
    --rm: bool
    --interactive(-i): bool
    --tty(-t): bool
    --detach(-d): bool
    --publish(-p): string
    --volume(-v): string
    --env(-e): string
    --network(-n): string@"nu-complete docker networks"
    --workdir(-w): string
    --user(-u): string
    --entrypoint: string
    --platform: string
    --pull: string
    --restart: string
    --rm: bool
]

export extern "docker exec" [
    container: string@"nu-complete docker containers running"
    ...commands: string
    --interactive(-i): bool
    --tty(-t): bool
    --detach(-d): bool
    --env(-e): string
    --workdir(-w): string
    --user(-u): string
]

export extern "docker build" [
    context?: string
    --tag(-t): string
    --file(-f): string
    --build-arg(-b): string
    --no-cache: bool
    --pull: bool
    --push: bool
    --platform: string
    --target: string
    --quiet(-q): bool
]

export extern "docker ps" [
    --all(-a): bool
    --filter(-f): string
    --format: string
    --latest(-l): bool
    --no-trunc: bool
    --quiet(-q): bool
    --size(-s): bool
]

export extern "docker images" [
    --all(-a): bool
    --digests: bool
    --filter(-f): string
    --format: string
    --no-trunc: bool
    --quiet(-q): bool
]

export extern "docker stop" [
    ...containers: string@"nu-complete docker containers running"
    --time(-t): int
]

export extern "docker start" [
    ...containers: string@"nu-complete docker containers"
    --attach(-a): bool
    --interactive(-i): bool
]

export extern "docker rm" [
    ...containers: string@"nu-complete docker containers"
    --force(-f): bool
    --volumes(-v): bool
]

export extern "docker rmi" [
    ...images: string@"nu-complete docker images"
    --force(-f): bool
    --no-prune: bool
]

export extern "docker logs" [
    container: string@"nu-complete docker containers"
    --follow(-f): bool
    --since: string
    --tail(-n): string
    --timestamps(-t): bool
]

export extern "docker inspect" [
    ...targets: string
    --format(-f): string
    --type: string
]

export extern "docker cp" [
    source: string
    dest: string
    --archive(-a): bool
    --follow(-L): bool
]

export extern "docker pull" [
    name: string@"nu-complete docker images"
    --all-tags(-a): bool
    --platform: string
    --quiet(-q): bool
]

export extern "docker push" [
    name: string@"nu-complete docker images"
    --all-tags(-a): bool
    --quiet(-q): bool
]

export extern "docker tag" [
    source: string@"nu-complete docker images"
    target: string
]

export extern "docker network" [
    subcommand: string
    ...args: string
]

export extern "docker volume" [
    subcommand: string
    ...args: string
]

export extern "docker container" [
    subcommand: string
    ...args: string
]

export extern "docker system" [
    subcommand: string
    ...args: string
    --filter: string
]

export extern "docker compose" [
    subcommand: string@"nu-complete docker compose subcommands"
    ...args: string
    --file(-f): string
    --project-name(-p): string
    --verbose(-v): bool
    --project-directory: string
    --profile: string
]

export extern "docker compose up" [
    ...services: string@"nu-complete docker compose services"
    --build: bool
    --detach(-d): bool
    --remove-orphans: bool
    --force-recreate: bool
    --no-deps: bool
    --scale: string
    --watch: bool
]

export extern "docker compose down" [
    ...services: string@"nu-complete docker compose services"
    --rmi: string
    --volumes(-v): bool
    --remove-orphans: bool
    --timeout(-t): int
]

export extern "docker compose start" [
    ...services: string@"nu-complete docker compose services"
]

export extern "docker compose stop" [
    ...services: string@"nu-complete docker compose services"
    --timeout(-t): int
]

export extern "docker compose restart" [
    ...services: string@"nu-complete docker compose services"
    --timeout(-t): int
]

export extern "docker compose logs" [
    ...services: string@"nu-complete docker compose services"
    --follow(-f): bool
    --tail(-n): string
    --since: string
]

export extern "docker compose ps" [
    --all(-a): bool
    --format: string
    --quiet(-q): bool
    --services: bool
    --filter: string
]

export extern "docker compose exec" [
    service: string@"nu-complete docker compose services"
    ...commands: string
    --detach(-d): bool
    --interactive(-i): bool
    --tty(-t): bool
    --user(-u): string
    --workdir(-w): string
    --env(-e): string
]

export extern "docker compose run" [
    service: string@"nu-complete docker compose services"
    ...commands: string
    --detach(-d): bool
    --interactive(-i): bool
    --tty(-t): bool
    --rm: bool
    --user(-u): string
    --workdir(-w): string
]

export extern "docker compose build" [
    ...services: string@"nu-complete docker compose services"
    --build-arg(-b): string
    --no-cache: bool
    --pull: bool
    --push: bool
    --quiet(-q): bool
]

export extern "docker compose pull" [
    ...services: string@"nu-complete docker compose services"
    --quiet(-q): bool
    --ignore-pull-failures: bool
    --include-deps: bool
]

export extern "docker compose images" [
    ...services: string@"nu-complete docker compose services"
    --quiet(-q): bool
]

export extern "docker compose config" [
    --quiet(-q): bool
    --format: string
    --services: bool
    --volumes: bool
    --profiles: bool
]

export extern "docker compose create" [
    ...services: string@"nu-complete docker compose services"
    --build: bool
    --force-recreate: bool
    --no-recreate: bool
    --scale: string
]

export extern "docker compose kill" [
    ...services: string@"nu-complete docker compose services"
    --signal(-s): string
]

export extern "docker compose rm" [
    ...services: string@"nu-complete docker compose services"
    --force(-f): bool
    --volumes(-v): bool
    --stop: bool
]

export extern "docker compose pause" [
    ...services: string@"nu-complete docker compose services"
]

export extern "docker compose unpause" [
    ...services: string@"nu-complete docker compose services"
]

export extern "docker compose scale" [
    ...services: string@"nu-complete docker compose services"
]

export extern "docker compose top" [
    ...services: string@"nu-complete docker compose services"
]

export extern "docker compose events" [
    ...services: string@"nu-complete docker compose services"
    --json: bool
    --since: string
    --until: string
]

export extern "docker compose port" [
    service: string@"nu-complete docker compose services"
    port: int
    --protocol: string
    --index: int
]

export extern "docker compose watch" [
    ...services: string@"nu-complete docker compose services"
]

export extern "docker compose version" []
