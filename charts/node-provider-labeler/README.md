# node-provider-labeler Helm Chart

![Version: 0.20.0](https://img.shields.io/badge/Version-0.20.0-informational?style=flat-square) ![Type: application](https://img.shields.io/badge/Type-application-informational?style=flat-square) ![AppVersion: 0.8.0](https://img.shields.io/badge/AppVersion-0.8.0-informational?style=flat-square)

Set Kubernetes Node metadata from cloud provider IDs.

## Configuration

By default, node-provider-labeler will label nodes with a `provider-id` key and
with the value set to the last component of the `spec.providerID` field.
Examples:

| Provider | Provider ID                                                                                         | provider-id Value                        |
|----------|-----------------------------------------------------------------------------------------------------|------------------------------------------|
| EKS      | aws://us-west-2/i-0abcdef1234567890                                                                 | i-0abcdef1234567890                      |
| GKE      | gce://my-project/us-central1-a/gke-cluster-1-default-pool-12345678-abc1                             | gke-cluster-1-default-pool-12345678-abc1 |
| AKS      | azure://subscriptions/00000000/resourceGroups/myrg/providers/Microsoft.Compute/virtualMachines/myVM | myvm                                     |
| Kind     | kind://podman/node-prov/node-prov-worker                                                            | node-prov-worker                         |

You can use the `templates` configuration in your values.yaml to customize the behavior.

``` shell
templates:
  labels:
    - key: instance-id
      value: "{:last}"
```

See the
[documentation](https://github.com/jossware/node-provider-labeler/blob/main/README.md#templates)
for more on templates.

## Installation

``` shell
helm install node-provider-labeler oci://ghcr.io/jossware/charts/node-provider-labeler \
    --namespace node-provider-labeler \
    --create-namespace
```

## Values

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| affinity | object | `{}` | Assign custom affinity rules to the deployment. |
| extraEnv | list | `[]` | [Environment variables](https://kubernetes.io/docs/tasks/inject-data-application/define-environment-variable-container/) for the controller container. |
| fullnameOverride | string | `""` | String to fully override `"node-provider-labeler.fullname"` |
| image.pullPolicy | string | `"IfNotPresent"` | The image pull policy |
| image.repository | string | `"ghcr.io/jossware/node-provider-labeler"` | The image to use in the controller deployment |
| image.tag | string | `"v0.8.0"` | The tag of the image |
| imagePullSecrets | list | `[]` | Secrets with credentials to pull images from a private registry |
| nameOverride | string | `""` | Provide a name in place of the default |
| nodeSelector | object | `{}` | Node selector. |
| podAnnotations | object | `{}` | Annotations to be added to the pods |
| podLabels | object | `{}` | Labels to be added to the pods |
| podSecurityContext | object | `{}` | Pod level security context |
| rbac.create | bool | `true` | Specifies whether RBAC roles and bindings should be created |
| readinessProbe | object | `{"httpGet":{"path":"/health","port":"http"}}` | Server readiness probe |
| readinessProbe.httpGet.path | string | `"/health"` | HTTP path for readiness probe |
| readinessProbe.httpGet.port | string | `"http"` | Port for readiness probe |
| replicaCount | int | `1` | The number of replicas to run |
| resources | object | `{}` | Resource limits and requests for the deployment |
| securityContext | object | `{}` | Container level security context |
| service.port | int | `8080` | Metrics service port |
| service.type | string | `"ClusterIP"` | Metrics service type |
| serviceAccount.annotations | object | `{}` | Annotations to add to the service account |
| serviceAccount.automount | bool | `true` | Automatically mount a ServiceAccount's API credentials? |
| serviceAccount.create | bool | `true` | Specifies whether a service account should be created |
| serviceAccount.name | string | `""` | If not set and create is true, a name is generated using the fullname template |
| templates | object | `{}` | Optionally define templates for labels and/or annotations. If not defined, the chart will create the default label and value |
| tolerations | list | `[]` | Tolerations for use with node taints. |
| volumeMounts | list | `[]` | Additional volumeMounts to add to the deployment. |
| volumes | list | `[]` | Additional volumes to add on the deployment. |

## Contributing

Please refer to [CONTRIBUTING.MD](../CONTRIBUTING.md).
