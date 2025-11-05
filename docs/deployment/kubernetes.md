# Kubernetes Deployment Guide

This guide covers deploying Atlas on Kubernetes and Azure Kubernetes Service (AKS).

## Table of Contents

- [Prerequisites](#prerequisites)
- [Container Image](#container-image)
- [Kubernetes Manifests](#kubernetes-manifests)
- [Deployment](#deployment)
- [Configuration Management](#configuration-management)
- [Secrets Management](#secrets-management)
- [Scheduled Jobs](#scheduled-jobs)
- [Monitoring and Logging](#monitoring-and-logging)
- [Scaling and Performance](#scaling-and-performance)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### Required Tools

- **kubectl**: Kubernetes CLI
- **Azure CLI**: For AKS deployments
- **Helm**: (Optional) For package management

Install tools:

```bash
# Install kubectl
curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl

# Install Azure CLI
curl -sL https://aka.ms/InstallAzureCLIDeb | sudo bash

# Install Helm (optional)
curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash
```

### Kubernetes Cluster

**Azure Kubernetes Service (AKS)**:
```bash
# Login to Azure
az login

# Create resource group
az group create --name atlas-rg --location eastus

# Create AKS cluster
az aks create \
  --resource-group atlas-rg \
  --name atlas-cluster \
  --node-count 2 \
  --node-vm-size Standard_D2s_v3 \
  --enable-managed-identity \
  --generate-ssh-keys

# Get credentials
az aks get-credentials --resource-group atlas-rg --name atlas-cluster

# Verify connection
kubectl get nodes
```

**Other Kubernetes** (GKE, EKS, on-prem):
```bash
# Configure kubectl context
kubectl config use-context your-cluster

# Verify connection
kubectl get nodes
```

## Container Image

### Build and Push to Azure Container Registry

```bash
# Create ACR
az acr create \
  --resource-group atlas-rg \
  --name atlasregistry \
  --sku Basic

# Login to ACR
az acr login --name atlasregistry

# Build and push image
docker build -t atlasregistry.azurecr.io/atlas:1.0.0 .
docker push atlasregistry.azurecr.io/atlas:1.0.0

# Attach ACR to AKS (for pulling images)
az aks update \
  --resource-group atlas-rg \
  --name atlas-cluster \
  --attach-acr atlasregistry
```

### Alternative: Use Docker Hub

```bash
# Build and push
docker build -t your-dockerhub-user/atlas:1.0.0 .
docker push your-dockerhub-user/atlas:1.0.0
```

## Kubernetes Manifests

### Namespace

Create `manifests/namespace.yaml`:

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: atlas
  labels:
    name: atlas
```

### ConfigMap

Create `manifests/configmap.yaml`:

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: atlas-config
  namespace: atlas
data:
  atlas.toml: |
    [application]
    name = "atlas"
    version = "1.0.0"
    log_level = "info"

    [openehr]
    base_url = "https://your-ehrbase-server.com/ehrbase/rest/openehr/v1"
    vendor = "ehrbase"
    auth_type = "basic"
    username = "atlas_user"
    password = "${ATLAS_OPENEHR_PASSWORD}"
    tls_verify = true
    timeout_seconds = 30

    [openehr.retry]
    max_retries = 3
    initial_delay_ms = 1000
    max_delay_ms = 30000
    backoff_multiplier = 2.0

    [openehr.query]
    template_ids = ["IDCR - Vital Signs.v1"]
    ehr_ids = []
    batch_size = 1000
    parallel_ehrs = 8

    [export]
    mode = "incremental"
    export_composition_format = "preserve"
    max_retries = 3
    retry_backoff_ms = 1000

    [cosmosdb]
    endpoint = "https://your-account.documents.azure.com:443/"
    key = "${ATLAS_COSMOS_KEY}"
    database_name = "openehr_data"
    control_container = "atlas_control"
    data_container_prefix = "compositions"
    partition_key = "/ehr_id"
    max_concurrency = 10
    request_timeout_seconds = 30

    [state]
    enable_checkpointing = true
    checkpoint_interval_seconds = 30

    [verification]
    enable_verification = false
    checksum_algorithm = "sha256"

    [logging]
    local_enabled = true
    local_path = "/var/log/atlas"
    local_rotation = "daily"
    local_max_size_mb = 100
    azure_enabled = false
```

### Secret

Create `manifests/secret.yaml`:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: atlas-secrets
  namespace: atlas
type: Opaque
stringData:
  openehr-password: "your-openehr-password"
  cosmos-key: "your-cosmos-db-key"
```

**Important**: Do not commit secrets to version control. Use one of these approaches:
- Manually create secrets: `kubectl create secret generic atlas-secrets --from-literal=openehr-password=xxx`
- Use Azure Key Vault with CSI driver (recommended for production)
- Use sealed-secrets or external-secrets operator

### CronJob

Create `manifests/cronjob.yaml`:

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: atlas-export
  namespace: atlas
spec:
  # Run daily at 2 AM UTC
  schedule: "0 2 * * *"
  
  # Keep last 3 successful and 1 failed job
  successfulJobsHistoryLimit: 3
  failedJobsHistoryLimit: 1
  
  # Don't start new job if previous is still running
  concurrencyPolicy: Forbid
  
  jobTemplate:
    spec:
      # Retry failed jobs up to 2 times
      backoffLimit: 2
      
      # Clean up completed jobs after 1 hour
      ttlSecondsAfterFinished: 3600
      
      template:
        metadata:
          labels:
            app: atlas
            component: export
        spec:
          restartPolicy: OnFailure
          
          containers:
          - name: atlas
            image: atlasregistry.azurecr.io/atlas:1.0.0
            imagePullPolicy: IfNotPresent
            
            command: ["atlas"]
            args:
              - "export"
              - "-c"
              - "/etc/atlas/atlas.toml"
            
            env:
            - name: ATLAS_OPENEHR_PASSWORD
              valueFrom:
                secretKeyRef:
                  name: atlas-secrets
                  key: openehr-password
            - name: ATLAS_COSMOS_KEY
              valueFrom:
                secretKeyRef:
                  name: atlas-secrets
                  key: cosmos-key
            
            volumeMounts:
            - name: config
              mountPath: /etc/atlas
              readOnly: true
            - name: logs
              mountPath: /var/log/atlas
            
            resources:
              requests:
                memory: "2Gi"
                cpu: "1000m"
              limits:
                memory: "4Gi"
                cpu: "2000m"
            
            securityContext:
              allowPrivilegeEscalation: false
              runAsNonRoot: true
              runAsUser: 1000
              readOnlyRootFilesystem: true
              capabilities:
                drop:
                - ALL
          
          volumes:
          - name: config
            configMap:
              name: atlas-config
          - name: logs
            emptyDir: {}
          
          securityContext:
            fsGroup: 1000
```

### One-time Job

Create `manifests/job.yaml` for manual exports:

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: atlas-export-manual
  namespace: atlas
spec:
  backoffLimit: 2
  ttlSecondsAfterFinished: 3600
  
  template:
    metadata:
      labels:
        app: atlas
        component: export
    spec:
      restartPolicy: OnFailure
      
      containers:
      - name: atlas
        image: atlasregistry.azurecr.io/atlas:1.0.0
        
        command: ["atlas"]
        args:
          - "export"
          - "-c"
          - "/etc/atlas/atlas.toml"
          - "--mode"
          - "full"  # Override for full export
        
        env:
        - name: ATLAS_OPENEHR_PASSWORD
          valueFrom:
            secretKeyRef:
              name: atlas-secrets
              key: openehr-password
        - name: ATLAS_COSMOS_KEY
          valueFrom:
            secretKeyRef:
              name: atlas-secrets
              key: cosmos-key
        
        volumeMounts:
        - name: config
          mountPath: /etc/atlas
          readOnly: true
        - name: logs
          mountPath: /var/log/atlas
        
        resources:
          requests:
            memory: "2Gi"
            cpu: "1000m"
          limits:
            memory: "4Gi"
            cpu: "2000m"
      
      volumes:
      - name: config
        configMap:
          name: atlas-config
      - name: logs
        emptyDir: {}
```

## Deployment

### Deploy All Resources

```bash
# Create namespace
kubectl apply -f manifests/namespace.yaml

# Create ConfigMap
kubectl apply -f manifests/configmap.yaml

# Create Secret (manually or from file)
kubectl create secret generic atlas-secrets \
  --namespace atlas \
  --from-literal=openehr-password=your-password \
  --from-literal=cosmos-key=your-key

# Deploy CronJob
kubectl apply -f manifests/cronjob.yaml

# Verify deployment
kubectl get all -n atlas
```

### Manual Export

```bash
# Run one-time job
kubectl apply -f manifests/job.yaml

# Watch job progress
kubectl get jobs -n atlas -w

# View logs
kubectl logs -n atlas job/atlas-export-manual -f
```

### Trigger CronJob Manually

```bash
# Create job from cronjob
kubectl create job -n atlas atlas-export-adhoc --from=cronjob/atlas-export

# Watch job
kubectl get jobs -n atlas -w

# View logs
kubectl logs -n atlas job/atlas-export-adhoc -f
```

## Configuration Management

### Update Configuration

```bash
# Edit ConfigMap
kubectl edit configmap atlas-config -n atlas

# Or update from file
kubectl apply -f manifests/configmap.yaml

# Restart CronJob (delete and recreate)
kubectl delete cronjob atlas-export -n atlas
kubectl apply -f manifests/cronjob.yaml
```

### Multiple Configurations

Create separate ConfigMaps for different templates:

```bash
# Create ConfigMap for vital signs
kubectl create configmap atlas-config-vitals \
  --namespace atlas \
  --from-file=atlas.toml=configs/vitals.toml

# Create ConfigMap for lab results
kubectl create configmap atlas-config-labs \
  --namespace atlas \
  --from-file=atlas.toml=configs/labs.toml

# Deploy separate CronJobs
kubectl apply -f manifests/cronjob-vitals.yaml
kubectl apply -f manifests/cronjob-labs.yaml
```

## Secrets Management

### Azure Key Vault with CSI Driver (Recommended)

Install CSI driver:

```bash
# Add Helm repo
helm repo add csi-secrets-store-provider-azure https://azure.github.io/secrets-store-csi-driver-provider-azure/charts

# Install driver
helm install csi-secrets-store-provider-azure/csi-secrets-store-provider-azure \
  --generate-name \
  --namespace kube-system
```

Create Azure Key Vault:

```bash
# Create Key Vault
az keyvault create \
  --name atlas-keyvault \
  --resource-group atlas-rg \
  --location eastus

# Add secrets
az keyvault secret set \
  --vault-name atlas-keyvault \
  --name openehr-password \
  --value "your-password"

az keyvault secret set \
  --vault-name atlas-keyvault \
  --name cosmos-key \
  --value "your-key"
```

Create SecretProviderClass:

```yaml
apiVersion: secrets-store.csi.x-k8s.io/v1
kind: SecretProviderClass
metadata:
  name: atlas-secrets
  namespace: atlas
spec:
  provider: azure
  parameters:
    usePodIdentity: "false"
    useVMManagedIdentity: "true"
    userAssignedIdentityID: "<your-identity-client-id>"
    keyvaultName: "atlas-keyvault"
    objects: |
      array:
        - |
          objectName: openehr-password
          objectType: secret
        - |
          objectName: cosmos-key
          objectType: secret
    tenantId: "<your-tenant-id>"
  secretObjects:
  - secretName: atlas-secrets
    type: Opaque
    data:
    - objectName: openehr-password
      key: openehr-password
    - objectName: cosmos-key
      key: cosmos-key
```

Update CronJob to use CSI driver:

```yaml
volumes:
- name: secrets
  csi:
    driver: secrets-store.csi.k8s.io
    readOnly: true
    volumeAttributes:
      secretProviderClass: "atlas-secrets"
```

## Scheduled Jobs

### Adjust Schedule

Edit CronJob schedule:

```yaml
spec:
  # Every 6 hours
  schedule: "0 */6 * * *"
  
  # Every day at 2 AM and 2 PM
  schedule: "0 2,14 * * *"
  
  # Every Monday at 3 AM
  schedule: "0 3 * * 1"
```

### Suspend CronJob

```bash
# Suspend (stop scheduling new jobs)
kubectl patch cronjob atlas-export -n atlas -p '{"spec":{"suspend":true}}'

# Resume
kubectl patch cronjob atlas-export -n atlas -p '{"spec":{"suspend":false}}'
```

## Monitoring and Logging

### View Logs

```bash
# List pods
kubectl get pods -n atlas

# View logs from latest job
kubectl logs -n atlas -l app=atlas --tail=100 -f

# View logs from specific pod
kubectl logs -n atlas atlas-export-28471234-abcde -f

# View logs from previous run
kubectl logs -n atlas atlas-export-28471234-abcde --previous
```

### Azure Monitor Integration

Enable Container Insights:

```bash
# Enable monitoring
az aks enable-addons \
  --resource-group atlas-rg \
  --name atlas-cluster \
  --addons monitoring
```

Update configuration to use Azure Log Analytics:

```toml
[logging]
azure_enabled = true
azure_tenant_id = "${AZURE_TENANT_ID}"
azure_client_id = "${AZURE_CLIENT_ID}"
azure_client_secret = "${AZURE_CLIENT_SECRET}"
azure_log_analytics_workspace_id = "${AZURE_LOG_ANALYTICS_WORKSPACE_ID}"
azure_dcr_immutable_id = "${AZURE_DCR_IMMUTABLE_ID}"
azure_dce_endpoint = "${AZURE_DCE_ENDPOINT}"
azure_stream_name = "Custom-AtlasExport_CL"
```

### Prometheus Metrics (Optional)

Add Prometheus annotations to pod template:

```yaml
template:
  metadata:
    annotations:
      prometheus.io/scrape: "true"
      prometheus.io/port: "9090"
      prometheus.io/path: "/metrics"
```

## Scaling and Performance

### Resource Tuning

Adjust resources based on workload:

```yaml
resources:
  requests:
    memory: "4Gi"  # Increase for large exports
    cpu: "2000m"
  limits:
    memory: "8Gi"
    cpu: "4000m"
```

### Parallel Exports

Run multiple CronJobs for different templates:

```bash
# Deploy separate CronJobs
kubectl apply -f manifests/cronjob-template1.yaml
kubectl apply -f manifests/cronjob-template2.yaml
kubectl apply -f manifests/cronjob-template3.yaml
```

### Node Affinity

Pin jobs to specific nodes:

```yaml
spec:
  template:
    spec:
      affinity:
        nodeAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            nodeSelectorTerms:
            - matchExpressions:
              - key: workload
                operator: In
                values:
                - atlas
```

## Troubleshooting

### Job Fails to Start

```bash
# Check job status
kubectl describe job atlas-export-manual -n atlas

# Check pod events
kubectl get events -n atlas --sort-by='.lastTimestamp'

# Check image pull
kubectl describe pod <pod-name> -n atlas | grep -A 10 Events
```

### Out of Memory

```bash
# Check pod resource usage
kubectl top pod -n atlas

# Increase memory limits
kubectl edit cronjob atlas-export -n atlas
# Update resources.limits.memory
```

### Configuration Issues

```bash
# Verify ConfigMap
kubectl get configmap atlas-config -n atlas -o yaml

# Verify Secret
kubectl get secret atlas-secrets -n atlas -o yaml

# Test configuration
kubectl run -it --rm debug \
  --image=atlasregistry.azurecr.io/atlas:1.0.0 \
  --namespace=atlas \
  --restart=Never \
  -- atlas validate-config -c /etc/atlas/atlas.toml
```

### Network Issues

```bash
# Test connectivity from pod
kubectl run -it --rm debug \
  --image=curlimages/curl \
  --namespace=atlas \
  --restart=Never \
  -- curl https://your-ehrbase-server.com

# Check DNS
kubectl run -it --rm debug \
  --image=busybox \
  --namespace=atlas \
  --restart=Never \
  -- nslookup your-ehrbase-server.com
```

---

For more information, see:
- [Standalone Deployment](standalone.md) - Binary deployment
- [Docker Deployment](docker.md) - Container deployment
- [User Guide](../user-guide.md) - Usage instructions
- [Configuration Guide](../configuration.md) - Configuration reference

