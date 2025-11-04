# Azure Log Analytics Setup Guide

This guide provides step-by-step instructions for setting up Azure Log Analytics integration with Atlas using the modern **Logs Ingestion API**.

## Overview

Atlas uses the Azure Monitor Logs Ingestion API to send structured logs to Azure Log Analytics. This requires:

1. **Azure Log Analytics Workspace** - Where logs are stored
2. **Data Collection Endpoint (DCE)** - Ingestion endpoint
3. **Custom Table** - Schema for Atlas logs
4. **Data Collection Rule (DCR)** - Routing and transformation rules
5. **Azure AD App Registration** - Authentication credentials
6. **Role Assignment** - Permissions for the app to send logs

## Prerequisites

- Azure subscription with appropriate permissions
- Azure CLI installed (`az` command)
- Owner or Contributor role on the resource group
- Ability to create Azure AD app registrations

## Step 1: Create Log Analytics Workspace

If you don't already have a Log Analytics workspace:

```bash
# Set variables
RESOURCE_GROUP="atlas-rg"
LOCATION="eastus"
WORKSPACE_NAME="atlas-logs"

# Create resource group (if needed)
az group create --name $RESOURCE_GROUP --location $LOCATION

# Create Log Analytics workspace
az monitor log-analytics workspace create \
  --resource-group $RESOURCE_GROUP \
  --workspace-name $WORKSPACE_NAME \
  --location $LOCATION

# Get workspace ID
WORKSPACE_ID=$(az monitor log-analytics workspace show \
  --resource-group $RESOURCE_GROUP \
  --workspace-name $WORKSPACE_NAME \
  --query customerId -o tsv)

echo "Workspace ID: $WORKSPACE_ID"
```

Save the `WORKSPACE_ID` - you'll need it for Atlas configuration.

## Step 2: Create Data Collection Endpoint (DCE)

```bash
DCE_NAME="atlas-dce"

# Create DCE
az monitor data-collection endpoint create \
  --name $DCE_NAME \
  --resource-group $RESOURCE_GROUP \
  --location $LOCATION \
  --public-network-access Enabled

# Get DCE endpoint URL
DCE_ENDPOINT=$(az monitor data-collection endpoint show \
  --name $DCE_NAME \
  --resource-group $RESOURCE_GROUP \
  --query logsIngestion.endpoint -o tsv)

echo "DCE Endpoint: $DCE_ENDPOINT"
```

Save the `DCE_ENDPOINT` - you'll need it for Atlas configuration.

## Step 3: Create Custom Table in Log Analytics

Atlas logs are sent to a custom table named `AtlasExport_CL` (the `_CL` suffix is required for custom logs).

### 3.1 Create Table Schema JSON

Create a file named `atlas-table-schema.json`:

```json
{
  "properties": {
    "schema": {
      "name": "AtlasExport_CL",
      "columns": [
        {
          "name": "TimeGenerated",
          "type": "datetime"
        },
        {
          "name": "OperationType",
          "type": "string"
        },
        {
          "name": "EhrId",
          "type": "string"
        },
        {
          "name": "TemplateId",
          "type": "string"
        },
        {
          "name": "CompositionCount",
          "type": "int"
        },
        {
          "name": "Status",
          "type": "string"
        },
        {
          "name": "ErrorMessage",
          "type": "string"
        },
        {
          "name": "DurationMs",
          "type": "long"
        }
      ]
    }
  }
}
```

### 3.2 Create the Table

```bash
# Get workspace resource ID
WORKSPACE_RESOURCE_ID=$(az monitor log-analytics workspace show \
  --resource-group $RESOURCE_GROUP \
  --workspace-name $WORKSPACE_NAME \
  --query id -o tsv)

# Create custom table
az rest --method PUT \
  --url "${WORKSPACE_RESOURCE_ID}/tables/AtlasExport_CL?api-version=2021-12-01-preview" \
  --body @atlas-table-schema.json
```

## Step 4: Create Data Collection Rule (DCR)

### 4.1 Create DCR JSON

Create a file named `atlas-dcr.json`:

```json
{
  "location": "eastus",
  "properties": {
    "dataCollectionEndpointId": "/subscriptions/YOUR_SUBSCRIPTION_ID/resourceGroups/atlas-rg/providers/Microsoft.Insights/dataCollectionEndpoints/atlas-dce",
    "streamDeclarations": {
      "Custom-AtlasExport_CL": {
        "columns": [
          {
            "name": "TimeGenerated",
            "type": "datetime"
          },
          {
            "name": "OperationType",
            "type": "string"
          },
          {
            "name": "EhrId",
            "type": "string"
          },
          {
            "name": "TemplateId",
            "type": "string"
          },
          {
            "name": "CompositionCount",
            "type": "int"
          },
          {
            "name": "Status",
            "type": "string"
          },
          {
            "name": "ErrorMessage",
            "type": "string"
          },
          {
            "name": "DurationMs",
            "type": "long"
          }
        ]
      }
    },
    "destinations": {
      "logAnalytics": [
        {
          "workspaceResourceId": "/subscriptions/YOUR_SUBSCRIPTION_ID/resourceGroups/atlas-rg/providers/Microsoft.OperationalInsights/workspaces/atlas-logs",
          "name": "atlasWorkspace"
        }
      ]
    },
    "dataFlows": [
      {
        "streams": [
          "Custom-AtlasExport_CL"
        ],
        "destinations": [
          "atlasWorkspace"
        ],
        "transformKql": "source",
        "outputStream": "Custom-AtlasExport_CL"
      }
    ]
  }
}
```

### 4.2 Update DCR JSON with Your Values

```bash
# Get subscription ID
SUBSCRIPTION_ID=$(az account show --query id -o tsv)

# Get DCE resource ID
DCE_RESOURCE_ID=$(az monitor data-collection endpoint show \
  --name $DCE_NAME \
  --resource-group $RESOURCE_GROUP \
  --query id -o tsv)

# Update the JSON file (using sed or manually)
sed -i "s|YOUR_SUBSCRIPTION_ID|$SUBSCRIPTION_ID|g" atlas-dcr.json
sed -i "s|/subscriptions/.*/dataCollectionEndpoints/atlas-dce|$DCE_RESOURCE_ID|g" atlas-dcr.json
sed -i "s|/subscriptions/.*/workspaces/atlas-logs|$WORKSPACE_RESOURCE_ID|g" atlas-dcr.json
```

### 4.3 Create the DCR

```bash
DCR_NAME="atlas-dcr"

az monitor data-collection rule create \
  --name $DCR_NAME \
  --resource-group $RESOURCE_GROUP \
  --location $LOCATION \
  --rule-file atlas-dcr.json

# Get DCR immutable ID
DCR_IMMUTABLE_ID=$(az monitor data-collection rule show \
  --name $DCR_NAME \
  --resource-group $RESOURCE_GROUP \
  --query immutableId -o tsv)

echo "DCR Immutable ID: $DCR_IMMUTABLE_ID"
```

Save the `DCR_IMMUTABLE_ID` - you'll need it for Atlas configuration.

## Step 5: Create Azure AD App Registration

```bash
APP_NAME="atlas-logging"

# Create app registration
APP_ID=$(az ad app create \
  --display-name $APP_NAME \
  --query appId -o tsv)

echo "App (Client) ID: $APP_ID"

# Create service principal
az ad sp create --id $APP_ID

# Create client secret
CLIENT_SECRET=$(az ad app credential reset \
  --id $APP_ID \
  --append \
  --query password -o tsv)

echo "Client Secret: $CLIENT_SECRET"
echo "IMPORTANT: Save this secret - it won't be shown again!"

# Get tenant ID
TENANT_ID=$(az account show --query tenantId -o tsv)

echo "Tenant ID: $TENANT_ID"
```

Save the `APP_ID` (client ID), `CLIENT_SECRET`, and `TENANT_ID` - you'll need them for Atlas configuration.

## Step 6: Grant Permissions

The app registration needs the "Monitoring Metrics Publisher" role on the DCR:

```bash
# Get DCR resource ID
DCR_RESOURCE_ID=$(az monitor data-collection rule show \
  --name $DCR_NAME \
  --resource-group $RESOURCE_GROUP \
  --query id -o tsv)

# Get service principal object ID
SP_OBJECT_ID=$(az ad sp show --id $APP_ID --query id -o tsv)

# Assign role
az role assignment create \
  --role "Monitoring Metrics Publisher" \
  --assignee-object-id $SP_OBJECT_ID \
  --assignee-principal-type ServicePrincipal \
  --scope $DCR_RESOURCE_ID
```

## Step 7: Configure Atlas

Update your `atlas.toml` configuration file:

```toml
[logging]
local_enabled = true
local_path = "/var/log/atlas"
local_rotation = "daily"
local_max_size_mb = 100

# Azure Log Analytics
azure_enabled = true
azure_tenant_id = "YOUR_TENANT_ID"
azure_client_id = "YOUR_APP_ID"
azure_client_secret = "YOUR_CLIENT_SECRET"
azure_log_analytics_workspace_id = "YOUR_WORKSPACE_ID"
azure_dcr_immutable_id = "YOUR_DCR_IMMUTABLE_ID"
azure_dce_endpoint = "YOUR_DCE_ENDPOINT"
azure_stream_name = "Custom-AtlasExport_CL"
```

Or use environment variables:

```bash
export AZURE_TENANT_ID="your-tenant-id"
export AZURE_CLIENT_ID="your-app-id"
export AZURE_CLIENT_SECRET="your-client-secret"
export AZURE_LOG_ANALYTICS_WORKSPACE_ID="your-workspace-id"
export AZURE_DCR_IMMUTABLE_ID="your-dcr-immutable-id"
export AZURE_DCE_ENDPOINT="https://your-dce.monitor.azure.com"
```

## Step 8: Verify Setup

Run Atlas and check that logs are being sent to Azure:

```bash
# Run Atlas
atlas export -c atlas.toml

# Query logs in Azure (wait a few minutes for ingestion)
az monitor log-analytics query \
  --workspace $WORKSPACE_ID \
  --analytics-query "AtlasExport_CL | take 10" \
  --output table
```

## Troubleshooting

### Authentication Errors

If you see authentication errors:

- Verify the tenant ID, client ID, and client secret are correct
- Ensure the app registration has the "Monitoring Metrics Publisher" role on the DCR
- Check that the client secret hasn't expired

### Logs Not Appearing

If logs aren't appearing in Azure:

- Wait 5-10 minutes for initial ingestion
- Verify the DCR immutable ID is correct
- Check the DCE endpoint URL is correct
- Ensure the stream name matches: `Custom-AtlasExport_CL`

### Permission Denied

If you see permission errors:

- Verify the role assignment was successful
- Ensure you're using the DCR immutable ID, not the resource ID
- Check that the service principal has been created

## Querying Logs

Once logs are flowing, you can query them using KQL (Kusto Query Language):

```kql
// All export operations
AtlasExport_CL
| where OperationType == "export_started" or OperationType == "export_completed"
| project TimeGenerated, OperationType, EhrId, TemplateId, CompositionCount, Status, DurationMs

// Errors only
AtlasExport_CL
| where Status == "error"
| project TimeGenerated, OperationType, EhrId, ErrorMessage

// Performance metrics
AtlasExport_CL
| where OperationType startswith "metric_"
| summarize avg(DurationMs), max(DurationMs) by OperationType
```

## Cost Considerations

- **Log Analytics Workspace**: Pay-as-you-go based on data ingestion (GB) and retention
- **Data Collection Endpoint**: No additional cost
- **Data Collection Rule**: No additional cost
- **Typical Atlas logging**: ~1-5 MB per 1000 compositions exported

See [Azure Monitor Pricing](https://azure.microsoft.com/pricing/details/monitor/) for details.

## Next Steps

- Set up alerts for export failures
- Create dashboards in Azure Monitor
- Configure log retention policies
- Integrate with Azure Sentinel for security monitoring

## References

- [Azure Monitor Logs Ingestion API](https://learn.microsoft.com/azure/azure-monitor/logs/logs-ingestion-api-overview)
- [Data Collection Rules](https://learn.microsoft.com/azure/azure-monitor/essentials/data-collection-rule-overview)
- [Custom Logs](https://learn.microsoft.com/azure/azure-monitor/logs/create-custom-table)

