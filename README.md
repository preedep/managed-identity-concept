# managed-identity-concept

This repository contains a simple Rust API server that demonstrates how to authenticate requests using Azure Managed Identity. The API server is built using the [actix-web](https://actix.rs/) framework and uses the [jsonwebtoken](
I've developed this project to understand how Managed Identity works in Azure and how to authenticate requests using the access token provided by Azure AD and reduce the complexity of managing secrets and api key rotation in the application.

```mermaid
sequenceDiagram
    autonumber

    participant Client as Client (Rust)
    participant IMDS as Azure Instance Metadata Service (IMDS)
    participant Hypervisor as Azure Fabric Controller (Hypervisor)
    participant AzureAD as Azure Active Directory (Azure AD)
    participant API as Rust API Server
    participant JWKS as Azure AD JWKS Keys

    Note over Client: **Step 1 - Request Access Token from IMDS**
    Client->>IMDS: GET /metadata/identity/oauth2/token <br> (resource=api://your-api-id)
    IMDS-->>Client: 401 Unauthorized (If no Managed Identity assigned)

    Note over IMDS: **Step 2 - Validate Managed Identity**
    IMDS->>Hypervisor: Verify if Managed Identity is enabled <br> (Check System/User Assigned Identity)
    Hypervisor-->>IMDS: ✅ Managed Identity exists

    Note over IMDS: **Step 3 - Request Token from Azure AD**
    IMDS->>AzureAD: Request access token for Managed Identity <br> (aud=api://your-api-id)
    
    Note over AzureAD: **Step 4 - Issue a Signed JWT Token**
    AzureAD-->>IMDS: Return Access Token <br> (Signed using Azure AD’s private key)

    Note over IMDS: **Step 5 - Return Token to Client**
    IMDS->>Client: Access Token Response <br> (Includes aud, exp, iss)

    Note over Client: **Step 6 - Call Rust API Server**
    Client->>API: GET /protected-endpoint <br> (Authorization: Bearer Token)

    Note over API: **Step 7 - Extract & Validate Token**
    API->>JWKS: Fetch Azure AD Public JWKS Keys
    JWKS-->>API: Return Public Key for Signature Validation
    API->>AzureAD: Validate Token Claims <br> (Check aud, iss, exp)

    Note over AzureAD: **Step 8 - Verify Token Signature**
    AzureAD-->>API: ✅ Token is valid

    Note over API: **Step 9 - Return Protected Data**
    API-->>Client: Return JSON Response 🎉
```

## Prerequisites

[Grant App Role to Managed Identity](https://learn.microsoft.com/en-us/graph/api/serviceprincipal-post-approleassignments?view=graph-rest-1.0&tabs=http#permissions)


Example powershell
```
$tenantId = '<your tenant id>'
$serverRoleId = '<your server role id (app role id)>'
$clientManagedIdentity = '<your client managed identity id (principal id)>'
$serverEnterpriseApp = '<your server enterprise app id (object id)>'

Connect-AzureAd -TenantId $tenantId

New-AzureADServiceAppRoleAssignment `
    -Id $serverRoleId `
    -PrincipalId $clientManagedIdentity `
    -ObjectId $clientManagedIdentity `
    -ResourceId $serverEnterpriseApp
```

$$clientManagedIdentity = '<your client managed identity id (principal id)>'
![Image in master](https://github.com/preedep/managed-identity-concept/blob/master/images/image1.png)
![Image in develop](https://github.com/preedep/managed-identity-concept/blob/develop/images/image1.png)



If you’re performing these operations programmatically using a service principal or managed identity, ensure that it has the necessary Microsoft Graph API application permissions granted and consented:
•	AppRoleAssignment.ReadWrite.All: Allows the app to read and write all application role assignments without a signed-in user.
•	Application.ReadWrite.All: Allows the app to read and write all applications and app registrations without a signed-in user.

These permissions are essential for creating and managing app role assignments via the Microsoft Graph API.  ￼

Assigning Microsoft Graph API Permissions:

To assign these permissions to your application:
1.	Navigate to the Azure portal and select Azure Active Directory.
2.	Choose App registrations, and select your application.
3.	Go to API permissions and click on Add a permission.
4.	Select Microsoft Graph, then choose Application permissions.
5.	Search for and select the required permissions (e.g., AppRoleAssignment.ReadWrite.All and Application.ReadWrite.All).
6.	After adding the permissions, click on Grant admin consent to consent to the permissions for your organization.

Ensure that the account performing these actions has the necessary administrative privileges to grant these permissions.

By ensuring that the executing identity has the appropriate Azure AD role and Microsoft Graph API permissions, you should be able to successfully run the New-AzureADServiceAppRoleAssignment cmdlet.

