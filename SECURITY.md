# Security Policy

## Supported Versions

We provide security updates for the following versions:

| Version | Supported          |
| ------- | ------------------ |
| 1.x.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability in Chasm, please report it responsibly.

### How to Report

**DO NOT** create a public GitHub issue for security vulnerabilities.

Instead, please report security vulnerabilities by emailing:

📧 **security@nervosys.ai**

### What to Include

Please include the following in your report:

1. **Description** of the vulnerability
2. **Steps to reproduce** the issue
3. **Potential impact** of the vulnerability
4. **Suggested fix** (if you have one)
5. **Your contact information** for follow-up questions

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your report within 48 hours
- **Assessment**: We will assess the vulnerability and determine its severity within 7 days
- **Updates**: We will keep you informed of our progress
- **Resolution**: We aim to resolve critical vulnerabilities within 30 days
- **Credit**: We will credit you in the security advisory (unless you prefer to remain anonymous)

### Disclosure Policy

- We follow a 90-day coordinated disclosure policy
- We will work with you to understand and resolve the issue
- We will publicly disclose the vulnerability after a fix is available
- We will credit reporters who follow responsible disclosure practices

## Security Best Practices

When using Chasm, we recommend:

### For Users

1. **Keep updated**: Always use the latest version
2. **Secure your database**: The SQLite database may contain sensitive chat history
3. **API security**: If running the API server, use appropriate network security
4. **Access control**: Limit who can access your Chasm installation

### For Self-Hosting

If you're running the Chasm API server:

1. **Use HTTPS**: Always use TLS in production
2. **Firewall**: Restrict access to trusted networks
3. **Authentication**: Enable authentication for sensitive operations
4. **Audit logs**: Monitor access logs for suspicious activity

### Environment Variables

Chasm may use the following environment variables. Keep them secure:

- `CSM_DATABASE_PATH` - Path to the database file
- `CSM_API_PORT` - API server port

## Known Security Considerations

### Data Storage

- Chat history is stored in a local SQLite database
- The database is not encrypted at rest by default
- Users are responsible for securing the database file

### API Server

- The API server binds to `0.0.0.0` by default (all interfaces)
- CORS is configured for localhost by default
- No authentication is required by default

### Cookie Decryption

- Chasm can decrypt browser cookies to access chat provider sessions
- This feature requires appropriate system permissions
- Use this feature responsibly and only on systems you own

## Security Audits

We welcome security audits of our codebase. If you're interested in conducting a security audit, please contact us at security@nervosys.ai.

## Bug Bounty

We currently do not have a formal bug bounty program. However, we appreciate and acknowledge security researchers who responsibly disclose vulnerabilities.

## Contact

For security-related questions or concerns:

- Email: security@nervosys.ai
- PGP Key: Available upon request

---

Thank you for helping keep Chasm and its users safe! 🛡️


