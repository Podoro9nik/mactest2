# Changes

## Desktop 3.0.1

- Fix Elyum photo uploads exceeding the MCP server's 4 MiB JSON request limit. Small PNG references stay lossless; larger pictures use bounded JPEG encoding before upload. Source files and Codex image preparation are unchanged.
- Use matching image MIME types and filename extensions, and distinguish upload failures from connection failures. Upload errors explain that generation has not started.
- Preserve preview review and explicit confirmation before purchasing an original.

Android 3.0.0 is unchanged by this desktop update.
