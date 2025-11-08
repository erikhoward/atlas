# Security Policy

## Supported Versions

We take security seriously and provide security updates for the following versions of Atlas:

| Version | Supported          |
| ------- | ------------------ |
| 2.0.x   | :white_check_mark: |
| 1.5.x   | :white_check_mark: |
| < 1.5   | :x:                |

We recommend always using the latest stable version to ensure you have the most recent security patches and improvements.

## Reporting a Vulnerability

We appreciate the security research community's efforts in helping us maintain the security of Atlas. If you discover a security vulnerability, please follow these guidelines:

### How to Report

**DO NOT** open a public GitHub issue for security vulnerabilities. Instead, please report security issues privately using one of the following methods:

1. **Email** (Preferred): Send details to **erikhoward@pm.me** with the subject line "Atlas Security Vulnerability Report"
2. **GitHub Security Advisories**: Use the [GitHub Security Advisory](https://github.com/erikhoward/atlas/security/advisories/new) feature to privately report vulnerabilities

### What to Include

To help us understand and address the vulnerability quickly, please include the following information in your report:

- **Description**: A clear description of the vulnerability and its potential impact
- **Steps to Reproduce**: Detailed steps to reproduce the issue, including any necessary configuration or setup
- **Affected Versions**: Which version(s) of Atlas are affected
- **Proof of Concept**: If possible, include a minimal proof of concept or example code
- **Suggested Fix**: If you have ideas on how to fix the vulnerability, please share them
- **Your Contact Information**: How we can reach you for follow-up questions

### What to Expect

- **Acknowledgement**: We will acknowledge receipt of your vulnerability report within **48 hours** of submission
- **Initial Assessment**: We will provide an initial assessment of the report within 5 business days
- **Regular Updates**: We will keep you informed of our progress as we investigate and address the issue
- **Credit**: With your permission, we will credit you in the security advisory and release notes when the fix is published

## Our Security Process

### Investigation and Patching

Once we receive a vulnerability report:

1. **Triage**: We will assess the severity and impact of the vulnerability
2. **Investigation**: Our team will investigate the issue and develop a fix
3. **Testing**: The fix will be thoroughly tested to ensure it resolves the issue without introducing new problems
4. **Coordination**: We will coordinate with you on the disclosure timeline

### Disclosure Timeline

We follow responsible disclosure practices:

- **Private Fix Development**: We will develop and test fixes privately
- **Advance Notice**: We aim to provide advance notice to affected users when possible
- **Public Disclosure**: We will publicly disclose the vulnerability after a fix is available, typically within 90 days of the initial report
- **Security Advisory**: We will publish a security advisory with details about the vulnerability and the fix

### Security Updates

Security fixes will be:

- Released as patch versions for supported versions (e.g., 2.0.1, 1.5.1)
- Documented in the [CHANGELOG.md](CHANGELOG.md)
- Announced via GitHub Security Advisories
- Tagged with appropriate CVE identifiers when applicable

## Security Best Practices

When deploying Atlas, we recommend following these security best practices:

### Configuration Security

- **Credentials**: Never commit credentials or API keys to version control
- **Environment Variables**: Use environment variables or secure configuration management for sensitive data
- **File Permissions**: Ensure configuration files containing sensitive information have appropriate file permissions (e.g., `chmod 600`)
- **Secrets Management**: Consider using dedicated secrets management solutions (e.g., Azure Key Vault, HashiCorp Vault)

### Network Security

- **TLS/SSL**: Always use TLS/SSL for connections to OpenEHR servers, Azure Cosmos DB, and Azure Monitor
- **Firewall Rules**: Configure appropriate firewall rules to restrict access to databases and APIs
- **Network Isolation**: Deploy Atlas in a secure network environment with proper network segmentation

### Access Control

- **Least Privilege**: Use service accounts with minimal required permissions
- **Azure RBAC**: Configure appropriate Azure Role-Based Access Control for Cosmos DB and Monitor access
- **Authentication**: Use Azure Managed Identity or service principals instead of connection strings when possible

### Monitoring and Auditing

- **Logging**: Enable comprehensive logging to detect and investigate security incidents
- **Monitoring**: Monitor Atlas logs for suspicious activity or errors
- **Regular Updates**: Keep Atlas and its dependencies up to date with the latest security patches

## Scope

This security policy applies to:

- The Atlas core application (this repository)
- Official Docker images published by the Atlas team
- Documentation and example configurations

This policy does not cover:

- Third-party dependencies (please report issues to their respective maintainers)
- Forks or modified versions of Atlas not maintained by the core team
- Issues in user-specific configurations or deployments

## Recognition

We believe in recognizing security researchers who help us improve Atlas. With your permission, we will:

- Credit you in the security advisory
- List you in our security acknowledgments
- Mention your contribution in release notes

If you prefer to remain anonymous, please let us know in your report.

## Questions?

If you have questions about this security policy or need clarification on the reporting process, please contact **erikhoward@pm.me**.

Thank you for helping keep Atlas and its users secure!
