# Security

Knowmark benefits from security features that are part of Rocket.

Namely, following headers are set by default:

> X-Content-Type-Options: nosniff<br/>
> X-Frame-Options: SAMEORIGIN<br/>
> Permissions-Policy: interest-cohort=()

Sample security files are included in the `./security` directory in the root of the project for CI/CD and reference.
**Make sure those files are removed for deployment.** Knowmark will generate new salts and keys at startup if they're missing.

- Removing password salt will render all currently stored passwords invalid.
- Removing user_auth keys will invalidate all JWTs (user sessions).

## JWT

[Json Web Tokens](https://jwt.io/) are used for user authentication. `RSA PS256` algorithm is used for encoding/decoding the tokens.

JWT payload contains:

- [`iat`](https://datatracker.ietf.org/doc/html/rfc7519#section-4.1.6) and [`exp`](https://datatracker.ietf.org/doc/html/rfc7519#section-4.1.4) fields
- `user` - UUID of authenticated user
- `role` - [Role](#user-roles) of authenticated user

### User roles

Knowmark supports following user roles:

- `0` - **None** - Unauthenticated user
- `1` - **Normal** - Normal, authenticated user
- `2` - **Author** - User who can add content
- `3` - **Admin** - User without any role restrictions

## Passwords

Passwords are hashed using [bcrypt](https://en.wikipedia.org/wiki/Bcrypt) with 15 cycles. Only password hashes are stored.

## TLS

TLS can be enabled via `tls` feature. If you do enable `tls`, rocket should be [configured](https://rocket.rs/v0.5-rc/guide/configuration/#tls) with proper certificates and keys.

## Public directory

All files contained within public directory (default: `./public`) will be made available by the backend. Make sure it doesn't contain any sensitive information.
