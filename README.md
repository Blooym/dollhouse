# Dollhouse

![Dollware Badge](.assets/88x31.png)

> [!CAUTION]  
> **This project is made for me, my needs, and my infrastructure.**
>
> No support will be offered for this software. Breaking changes to functionalty or features may be made any time.

Server for creating file share links and embedding media on websites. 🎀

## Features

- **Upload auto-expiry**: Automatically delete uploads based how long it has been since they were last accessed (or modified on systems that don't support access times).

- **Storage-efficiency**: Uploads are deduplicated by storing them as a hash of their contents. Hashes are salted with an app-wide secret to prevent identification (as long as your app secret is secure).

- **Encrypted at rest**: All uploads are encrypted by the server when stored. The decryption key is attached to the returned share url and is not kept by the server. No upload can be accessed without the decryption key, even with access to the filesystem.
  - Note: encyption and decryption are handled server-side, anybody with access to the server network could intercept data unencrypted or read decryption keys from logs. While an unfortunate drawback, this is an accepted flaw as it allows uploads from clients that may otherwise be unable to encrypt before upload.

- **Multiple supported storage providers**: Uploads can be stored on the local filesystem, an S3 bucket, or even ephemeral process memory.

- **EXIF auto-removal**: Whenever possible identifiable EXIF data is stripped from uploads for better user privacy. Please note that this does not work on all file types and is done on a best-effort basis. If you need a guarantee that no EXIF data is present, you should strip it before uploading.

- **Domain-randomization support**: Return a random domain from a provided list on each upload, completely useless and entirely a for-fun feature.

## Setup

### Docker

1. Copy [compose.yml](./compose.yml) to a local file named `compose.yml` or add the
   service to your existing stack and fill in the environment variables.
   Information about configuration options can be found in the
   [configuration](#configuration) section.

2. Start the stack

```
docker compose up -d
```

### Manual

1. Ensure you have [Rust](https://www.rust-lang.org/tools/install) installed and
   in your `$PATH`.
2. Install the project binary

```
cargo install --git https://github.com/Blooym/dollhouse.git
```

3. Set configuration values as necessary.
   Information about configuration options can be found in the
   [configuration](#configuration) section.

```
dollhouse
```

## Configuration

Dollhouse is configured via command-line flags or environment variables and has full support for loading from `.env` files. Below is a list of all supported configuration options. You can also run `dollhouse --help` to get an up-to-date including default values.

| Name               | Description                                                                                                                                                                                                                                                                                                                                                                             | Flag                  | Env                           | Default                 |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- | ----------------------------- | ----------------------- |
| Address            | Internet socket address that the server should run on.                                                                                                                                                                                                                                                                                                                                  | `--address`           | `DOLLHOUSE_ADDRESS`           | `127.0.0.1:8731`        |
| Public URL(S)      | Base URL(s) to use when generating links to uploads. This affects link generation only; you are responsible for configuring any reverse proxy.                                                                                                                                                                                                                                          | `--public-urls`       | `DOLLHOUSE_PUBLIC_URLS`       | `http://127.0.0.1:8731` |
| Tokens             | One or more bearer tokens used for accessing authenticated endpoints. Multiple tokens can be provided, separated by commas.                                                                                                                                                                                                                                                             | `--tokens`            | `DOLLHOUSE_TOKENS`            |                         |
| Storage Provider   | Specifies the backend used for storing persistent data. Available options depend on compile-time features: `memory://` (in-memory), `fs://<path>` (filesystem), and `s3://bucket` (Simple Storage Service). When using S3, configuration is loaded according to the [AWS SDK credential provider chain](https://docs.aws.amazon.com/sdkref/latest/guide/standardized-credentials.html). | `--storage`           | `DOLLHOUSE_STORAGE_PROVIDER`  |                         |
| App Secret         | A unique secret used for hashing operations.                                                                                                                                                                                                                                                                                                                                            | `--app-secret`        | `DOLLHOUSE_APP_SECRET`        |                         |
| Upload Expiry Time | Duration of inactivity after which a file is automatically purged from storage. Accepts human-readable durations (e.g., `30min`, `1day`). If not set, files do not expire.                                                                                                                                                                                                              | `--upload-expiry`     | `DOLLHOUSE_UPLOAD_EXPIRY`     |                         |
| Upload Size Limit  | Maximum size of a single uploaded file. Accepts human-readable sizes (e.g., `50MB`, `1GB`).                                                                                                                                                                                                                                                                                             | `--upload-size-limit` | `DOLLHOUSE_UPLOAD_SIZE_LIMIT` | `50MB`                  |
| Upload Mimetypes   | List of allowed MIME types for uploads. Supports wildcards (e.g., `image/*`, `*/*`). File types are determined based on content (magic number detection). If detection fails and `*/*` is not allowed, the file is rejected. If `*/*` is allowed, the MIME type falls back to `application/octet-stream`.                                                                               | `--upload-mimetypes`  | `DOLLHOUSE_UPLOAD_MIMETYPES`  | `image/*`, `video/*`    |
