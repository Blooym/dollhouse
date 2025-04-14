# Dollhouse

![Dollware Badge](.assets/88x31.png)

> [!CAUTION]  
> **This project is made for me, my needs, and my infrastructure.**
>
> No support will be offered for this software. Breaking changes to functionalty or features may be made any time.

Server for creating file share links and embedding media on websites. 🎀

## Features

- **Upload auto-expiry**: Automatically delete uploads based how long it has been since they were last accessed.

- **Storage-efficiency**: Uploads are deduplicated by storing them as a hash of their contents; Hashes are then salted with a persistent key generated on first-time startup.

- **Encrypted at rest**: All uploads are encrypted by the server when stored. The decryption key is attached to the returned share url and is not kept by the server. No upload can be accessed without the decryption key, even with access to the filesystem.
  - Note: encyption and decryption are handled server-side, anybody with access to the server network could intercept data unencrypted or or decryption keys from logs. This is intentional as it allows uploads to embed on all websites. 

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

| Name               | Description                                                                                                                                                                                                                                                                                                                             | Flag                  | Env                           | Default                       |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------- | ----------------------------- | ----------------------------- |
| Address            | Internet socket address that the server should be ran on.                                                                                                                                                                                                                                                                               | `--address`           | `DOLLHOUSE_ADDRESS`           | `127.0.0.1:8731`              |
| Public URL         | Base url to use when generating links to uploads. This is only for link generation, you'll need to handle the reverse proxy yourself.                                                                                                                                                                                                   | `--public-url`        | `DOLLHOUSE_PUBLIC_URL`        | `http://127.0.0.1:8731`       |
| Tokens             | One or more bearer tokens to use when interacting with authenticated endpoints.                                                                                                                                                                                                                                                         | `--tokens`            | `DOLLHOUSE_TOKENS`            |                               |
| Data path          | Path to the directory where data should be stored. This directory should not be used for anything else as it and all subdirectories will be automatically managed.                                                                                                                                                                      | `--data-path`         | `DOLLHOUSE_DATA_PATH`         | `OS Data Directory/dollhouse` |
| Upload expiry time | Time since last access before a file is automatically purged from storage. No value means files will never expire.                                                                                                                                                                                                                      | `--upload-expiry`     | `DOLLHOUSE_UPLOAD_EXPIRY`     |                               |
| Upload size limit  | Maximum file size that can be uploaded.                                                                                                                                                                                                                                                                                                 | `--upload-size-limit` | `DOLLHOUSE_UPLOAD_SIZE_LIMIT` | `50MB`                        |
| Upload mimetypes   | File mimetypes that can be uploaded. Supports type wildcards (e.g. 'image/*', '*/*'). MIME types are determined by the magic numbers of uploaded content, if the mimetype cannot be determined the file will be rejected unless all mimetypes are allowed, where it instead will be uploaded as `.unknown` and sent via `octet-stream`. | `--upload-mimetypes`  | `DOLLHOUSE_UPLOAD_MIMETYPES`  | `image/*`, `video/*`          |
