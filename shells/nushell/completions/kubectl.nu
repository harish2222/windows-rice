# Kubernetes custom completions for Nushell

# ── Namespace names ──
export def "nu-complete k8s namespaces" [] {
    ^kubectl get namespaces -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |n| { value: $n description: "namespace" } }
}

# ── Pod names ──
export def "nu-complete k8s pods" [] {
    ^kubectl get pods -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |p| { value: $p description: "pod" } }
}

# ── Pod names in current namespace ──
export def "nu-complete k8s pods ns" [] {
    ^kubectl get pods -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |p| { value: $p description: "pod" } }
}

# ── Service names ──
export def "nu-complete k8s services" [] {
    ^kubectl get services -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |s| { value: $s description: "service" } }
}

# ── Deployment names ──
export def "nu-complete k8s deployments" [] {
    ^kubectl get deployments -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |d| { value: $d description: "deployment" } }
}

# ── StatefulSet names ──
export def "nu-complete k8s statefulsets" [] {
    ^kubectl get statefulsets -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |s| { value: $s description: "statefulset" } }
}

# ── ConfigMap names ──
export def "nu-complete k8s configmaps" [] {
    ^kubectl get configmaps -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |c| { value: $c description: "configmap" } }
}

# ── Secret names ──
export def "nu-complete k8s secrets" [] {
    ^kubectl get secrets -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |s| { value: $s description: "secret" } }
}

# ── Node names ──
export def "nu-complete k8s nodes" [] {
    ^kubectl get nodes -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |n| { value: $n description: "node" } }
}

# ── Context names ──
export def "nu-complete k8s contexts" [] {
    ^kubectl config get-contexts -o name 2>/dev/null | lines | each { |c| { value: $c description: "context" } }
}

# ── Storage class names ──
export def "nu-complete k8s storageclasses" [] {
    ^kubectl get storageclasses -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |s| { value: $s description: "storageclass" } }
}

# ── Ingress names ──
export def "nu-complete k8s ingresses" [] {
    ^kubectl get ingresses -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |i| { value: $i description: "ingress" } }
}

# ── PersistentVolumeClaim names ──
export def "nu-complete k8s pvcs" [] {
    ^kubectl get pvcs -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |p| { value: $p description: "pvc" } }
}

# ── DaemonSet names ──
export def "nu-complete k8s daemonsets" [] {
    ^kubectl get daemonsets -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |d| { value: $d description: "daemonset" } }
}

# ── Job names ──
export def "nu-complete k8s jobs" [] {
    ^kubectl get jobs -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |j| { value: $j description: "job" } }
}

# ── CronJob names ──
export def "nu-complete k8s cronjobs" [] {
    ^kubectl get cronjobs -o jsonpath='{.items[*].metadata.name}' 2>/dev/null | split row ' ' | each { |c| { value: $c description: "cronjob" } }
}

# ── Kubectl subcommands ──
export def "nu-complete k8s subcommands" [] {
    [
        { value: "annotate" description: "Update the annotations on a resource" }
        { value: "api-resources" description: "API resources on the server" }
        { value: "api-versions" description: "API versions on the server" }
        { value: "apply" description: "Apply a configuration to a resource" }
        { value: "attach" description: "Attach to a running container" }
        { value: "auth" description: "Inspect authorization" }
        { value: "autoscale" description: "Create a horizontal pod autoscaler" }
        { value: "certificate" description: "Modify certificate resources" }
        { value: "cluster-info" description: "Display cluster info" }
        { value: "completion" description: "Output shell completion code" }
        { value: "config" description: "Modify kubeconfig files" }
        { value: "convert" description: "Convert config files between different API versions" }
        { value: "cordon" description: "Mark node as unschedulable" }
        { value: "cp" description: "Copy files and directories to and from containers" }
        { value: "create" description: "Create a resource from a file or stdin" }
        { value: "delete" description: "Delete resources by file names, stdin, resources and names" }
        { value: "describe" description: "Show details of a specific resource or group" }
        { value: "diff" description: "Diff current live vs. possibly desired configuration" }
        { value: "drain" description: "Drain node in preparation for maintenance" }
        { value: "edit" description: "Edit a resource on the server" }
        { value: "exec" description: "Execute a command in a container" }
        { value: "explain" description: "Documentation of resources" }
        { value: "expose" description: "Take a replication controller, service, deployment, or pod and expose it as a new Kubernetes Service" }
        { value: "get" description: "Display one or many resources" }
        { value: "kustomize" description: "Build a kustomization target from a directory" }
        { value: "label" description: "Update the labels on a resource" }
        { value: "logs" description: "Print the logs for a container in a pod" }
        { value: "patch" description: "Update field(s) of a resource using strategic merge patch" }
        { value: "port-forward" description: "Forward one or more local ports to a pod" }
        { value: "proxy" description: "Run a proxy to the Kubernetes API server" }
        { value: "replace" description: "Replace a resource by file name or stdin" }
        { value: "rollout" description: "Manage the rollout of a resource" }
        { value: "run" description: "Run a particular image on the cluster" }
        { value: "scale" description: "Set a new size for a deployment, replica set, job, or replication controller" }
        { value: "secrets" description: "Manage secrets" }
        { value: "taint" description: "Update the taints on one or more nodes" }
        { value: "top" description: "Display resource (CPU/memory) usage" }
        { value: "uncordon" description: "Mark node as schedulable" }
        { value: "version" description: "Print the client and server version information" }
        { value: "wait" description: "Wait for a specific condition on one or many resources" }
    ]
}

# ── Resource types ──
export def "nu-complete k8s resources" [] {
    [
        { value: "pods" description: "pods" }
        { value: "services" description: "services" }
        { value: "deployments" description: "deployments" }
        { value: "statefulsets" description: "statefulsets" }
        { value: "daemonsets" description: "daemonsets" }
        { value: "configmaps" description: "configmaps" }
        { value: "secrets" description: "secrets" }
        { value: "namespaces" description: "namespaces" }
        { value: "nodes" description: "nodes" }
        { value: "persistentvolumes" description: "persistentvolumes" }
        { value: "persistentvolumeclaims" description: "persistentvolumeclaims" }
        { value: "ingresses" description: "ingresses" }
        { value: "jobs" description: "jobs" }
        { value: "cronjobs" description: "cronjobs" }
        { value: "storageclasses" description: "storageclasses" }
        { value: "replicasets" description: "replicasets" }
        { value: "networkpolicies" description: "networkpolicies" }
        { value: "serviceaccounts" description: "serviceaccounts" }
        { value: "clusterroles" description: "clusterroles" }
        { value: "clusterrolebindings" description: "clusterrolebindings" }
        { value: "roles" description: "roles" }
        { value: "rolebindings" description: "rolebindings" }
    ]
}

# ── Extern definitions ──
export extern "kubectl" [
    subcommand: string@"nu-complete k8s subcommands"
    ...args: string
    --namespace(-n): string@"nu-complete k8s namespaces"
    --context: string@"nu-complete k8scontexts"
    --cluster: string
    --user: string
    --kubeconfig: string
    --as: string
    --as-group: string
    --as-uid: string
    --cache-dir: string
    --certificate-authority: string
    --client-certificate: string
    --client-key: string
    --insecure-skip-tls-verify: bool
    --server: string
    --token: string
    --tls-server-name: string
]

export extern "kubectl get" [
    resource: string@"nu-complete k8s resources"
    ...names: string
    --all-namespaces(-A): bool
    --namespace(-n): string@"nu-complete k8s namespaces"
    --output(-o): string
    --watch(-w): bool
    --selector(-l): string
    --field-selector: string
    --sort-by: string
    --no-headers: bool
    --show-kind: bool
    --show-labels: bool
    --output-wide(-w): bool
]

export extern "kubectl describe" [
    resource: string@"nu-complete k8s resources"
    ...names: string
    --all-namespaces(-A): bool
    --namespace(-n): string@"nu-complete k8s namespaces"
    --selector(-l): string
    --filename(-f): string
]

export extern "kubectl logs" [
    pod: string@"nu-complete k8s pods"
    container?: string
    --all-containers(-a): bool
    --follow(-f): bool
    --previous(-p): bool
    --since: string
    --tail(-n): int
    --timestamps(-t): bool
    --namespace(-n): string@"nu-complete k8s namespaces"
]

export extern "kubectl exec" [
    pod: string@"nu-complete k8s pods"
    ...commands: string
    --container(-c): string
    --interactive(-i): bool
    --tty(-t): bool
    --namespace(-n): string@"nu-complete k8s namespaces"
]

export extern "kubectl delete" [
    resource: string@"nu-complete k8s resources"
    ...names: string
    --filename(-f): string
    --force: bool
    --grace-period: int
    --ignore-not-found: bool
    --namespace(-n): string@"nu-complete k8s namespaces"
    --selector(-l): string
]

export extern "kubectl apply" [
    filename(-f)?: string
    --filename(-f): string
    --recursive(-R): bool
    --force-conflicts: bool
    --server-side: bool
    --prune: bool
    --selector(-l): string
    --namespace(-n): string@"nu-complete k8s namespaces"
]

export extern "kubectl create" [
    resource: string@"nu-complete k8s resources"
    --filename(-f): string
    --dry-run: string
    --namespace(-n): string@"nu-complete k8s namespaces"
    --output(-o): string
    --save-config: bool
    --edit: bool
]

export extern "kubectl rollout" [
    subcommand: string
    resource: string@"nu-complete k8s resources"
    ...names: string
    --namespace(-n): string@"nu-complete k8s namespaces"
]

export extern "kubectl scale" [
    resource: string@"nu-complete k8s resources"
    ...names: string
    --replicas(-r): int
    --current-replicas: int
    --namespace(-n): string@"nu-complete k8s namespaces"
]

export extern "kubectl port-forward" [
    resource: string@"nu-complete k8s pods"
    ...ports: string
    --namespace(-n): string@"nu-complete k8s namespaces"
]

export extern "kubectl config" [
    subcommand: string
    ...args: string
]

export extern "kubectl config use-context" [
    context: string@"nu-complete k8s contexts"
]

export extern "kubectl config get-contexts" [
    ...args: string
    --output(-o): string
]

export extern "kubectl top" [
    subcommand: string
    ...names: string
    --namespace(-n): string@"nu-complete k8s namespaces"
    --containers(-c): bool
    --sort-by: string
]

export extern "kubectl label" [
    resource: string@"nu-complete k8s resources"
    ...names: string
    --overwrite: bool
    --dry-run: string
    --namespace(-n): string@"nu-complete k8s namespaces"
]

export extern "kubectl annotate" [
    resource: string@"nu-complete k8s resources"
    ...names: string
    --overwrite: bool
    --dry-run: string
    --namespace(-n): string@"nu-complete k8s namespaces"
]

export extern "kubectl cordon" [
    ...nodes: string@"nu-complete k8s nodes"
]

export extern "kubectl uncordon" [
    ...nodes: string@"nu-complete k8s nodes"
]

export extern "kubectl drain" [
    ...nodes: string@"nu-complete k8s nodes"
    --delete-emptydir-data: bool
    --force: bool
    --ignore-daemonsets: bool
    --grace-period: int
    --timeout: int
]

export extern "kubectl edit" [
    resource: string@"nu-complete k8s resources"
    ...names: string
    --namespace(-n): string@"nu-complete k8s namespaces"
    --output(-o): string
]

export extern "kubectl wait" [
    --for: string
    --timeout: int
    resource: string@"nu-complete k8s resources"
    ...names: string
    --namespace(-n): string@"nu-complete k8s namespaces"
]

export extern "kubectl explain" [
    resource: string@"nu-complete k8s resources"
    --api-version(-a): string
    --recursive(-R): bool
]

export extern "kubectl cp" [
    source: string
    dest: string
    --container(-c): string
    --namespace(-n): string@"nu-complete k8s namespaces"
]

export extern "kubectl patch" [
    resource: string@"nu-complete k8s resources"
    ...names: string
    --type(-t): string
    --patch(-p): string
    --filename(-f): string
    --dry-run: string
    --namespace(-n): string@"nu-complete k8s namespaces"
]
