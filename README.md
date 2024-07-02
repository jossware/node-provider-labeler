![build](https://github.com/jossware/node-provider-labeler/actions/workflows/build.yaml/badge.svg)  ![release](https://github.com/jossware/node-provider-labeler/actions/workflows/release.yaml/badge.svg)  ![chart](https://github.com/jossware/node-provider-labeler/actions/workflows/chart.yaml/badge.svg)

# node-provider-labeler

node-provider-labeler is an open-source Kubernetes controller that monitors
Kubernetes Nodes for the `spec.providerID` field, which cloud providers
[populate](https://kubernetes.io/docs/reference/kubernetes-api/cluster-resources/node-v1/#NodeSpec)
in the format: `<ProviderName>://<ProviderSpecificNodeID>`. When
`node-provider-labeler` detects a `spec.providerID` field, it extracts
(user-defined) information from the value and propagates that to Kubernetes
metadata (a label or an annotation or both) on the `Node` resource.

## Features

- **Cloud Provider Integration**: Can flexibly handle `spec.providerID` values
  set by various cloud providers.
- **Information Extraction**: Parses the provider-specific information from the
  `spec.providerID` field in a flexible and configurable fashion.
- **Node Labeling**: Adds the extracted information as a label or annotation on
  the Kubernetes Node resource for easy identification and management.
- **Rust Implementation**: Leverages Rust's performance and safety features for
  robust and efficient operation.

## Uses

- **Cluster Management**: Simplifies the management and organization of nodes in
  a Kubernetes cluster by automatically setting node metadata based on their
  cloud provider IDs.
- **Monitoring and Automation**: Enhances monitoring and automation tools that
  rely on node metadata, allowing for more precise targeting and actions.

## Install

node-provider-labeler runs as a Kubernetes deployment so you can manage and
deploy it in the same way you as your other Kubernetes workloads. We do provide
a [helm](https://helm.sh/) chart and a [kustomize](https://kustomize.io/) base
to make it easy to get started.

### Deploy via Helm

``` shell
helm install node-provider-labeler oci://ghcr.io/jossware/charts/node-provider-labeler
```

### Deploy via Kustomize

``` shell
kustomize build https://github.com/jossware/node-provider-labeler.git/kustomize \
    | kubectl apply -f -
```

## Run

By default, node-provider-labeler will label nodes with a `provider-id` key and
with the value set to the last component of the `spec.providerID` field.
Examples:

| Provider | Provider ID                                                                                         | provider-id Value                        |
|----------|-----------------------------------------------------------------------------------------------------|------------------------------------------|
| EKS      | aws://us-west-2/i-0abcdef1234567890                                                                 | i-0abcdef1234567890                      |
| GKE      | gce://my-project/us-central1-a/gke-cluster-1-default-pool-12345678-abc1                             | gke-cluster-1-default-pool-12345678-abc1 |
| AKS      | azure://subscriptions/00000000/resourceGroups/myrg/providers/Microsoft.Compute/virtualMachines/myVM | myvm                                     |
| Kind     | kind://podman/node-prov/node-prov-worker                                                            | node-prov-worker                         |

If you want to change the label key or value, use the `--label` flag when
starting the controller.

``` shell
  -l, --label <LABEL>
          The label key and optional template to use for the label value.
          The default is "provider-id={:last}" if there are no other labels or annotations configured.
          Repeat to add multiple labels.
```

Examples:
* --label=label-key
* --label=label-key={:last} --label=other-label-key={0}-{1}

See [Templates](#templates) for more on how to define your label values.

If you'd rather annotate your node, use the `--annotation` flag:

``` shell
  -a, --annotation <ANNOTATION>
          The annotation key and optional template to use for the annotation value
          Repeat to add multiple annotations.
```

Examples:
* --annotation=annotation-key
* --annotation=annotation-key={:last} --annotation=other-annotation-key={0}-{1}

You can use both the `--label` and `--annotation` flag(s) if you want to label
_and_ annotate your nodes.

node-provider-labeler watches for `Node` resource events and reconciles metadata
immediately. It will also periodically reconcile `Node`s (every hour by
default). You can change that interval with the `--requeue-duration` flag:

``` shell
      --requeue-duration <REQUEUE_DURATION>
          Requeue reconciliation of a node after this duration in seconds

          [default: 3600]
```

## Templates

You can write a string template to define how you want information extracted
from `.spec.providerID` and into your metadata value. node-id-labeler parses the
`providerID` and makes discovered information available via tokens you can use
in your templates. It splits the value of the ID (the part after the provider
"protocol") by "/" and it is possible to access individual parts by index or by
named helpers.

node-provider-labeler will sometimes generate different values depending on
whether you are configuring a label or annotation (because of the differences in
allowed characters in
[label](https://kubernetes.io/docs/concepts/overview/working-with-objects/labels/#syntax-and-character-set)
or
[annotation](https://kubernetes.io/docs/concepts/overview/working-with-objects/annotations/#syntax-and-character-set)
values).

Let's take a look at a concrete example for AWS: "aws://us-west-2/i-0abcdef1234567890". 

| Token       | Label Value                   | Annotation Value              |
|-------------|-------------------------------|-------------------------------|
| {:provider} | aws                           | aws                           |
| {:last}     | i-0abcdef1234567890           | i-0abcdef1234567890           |
| {:first}    | us-west-2                     | us-west-2                     |
| {:all}      | us-west-2_i-0abcdef1234567890 | us-west-2/i-0abcdef1234567890 |
| {0}         | us-west-2                     | us-west-2                     |
| {1}         | i-0abcdef1234567890           | i-0abcdef1234567890           |

Of course, you can combine tokens to define your value. Examples:

| Template            | Label Value                      | Annotation Value                 |
|---------------------|----------------------------------|----------------------------------|
| {:provider}-{:last} | aws-i-0abcdef1234567890          | aws-i-0abcdef1234567890          |
| {:first}-{:last}    | us-west-2-i-0abcdef1234567890    | us-west-2-i-0abcdef1234567890    |
| id_{:all}           | id_us-west-2_i-0abcdef1234567890 | id_us-west-2/i-0abcdef1234567890 |
