# Privacy Policy — NaBrowse

NaBrowse is a privacy-first web browser. This document describes how we
protect your privacy and what data (if any) we collect.

## Core principles

1. **No telemetry.** We do not collect, transmit, or store any data about your
   browsing activity, system, or usage patterns.
2. **No tracking.** There are no trackers, no analytics beacons, no
   fingerprinting, and no third-party services embedded in the browser.
3. **No phone-home.** The network stack only initiates connections to URLs you
   explicitly navigate to. It never contacts NaBrowse servers for updates,
   telemetry, or any other purpose.
4. **Local-first.** All HTML parsing, CSS computation, layout, painting, and
   JavaScript execution happens entirely on your machine. No browsing data is
   sent to a remote server.
5. **No user accounts.** NaBrowse does not require or support user accounts,
   sign-in, or sync. Your browsing stays on your device.

## Network connections

The only network connections NaBrowse makes are:

- HTTP/HTTPS requests to URLs you type or click in the browser.
- DNS resolution for those URLs (via your operating system's resolver).
- TLS handshakes for HTTPS sites (via rustls, using system root certificates).

No connections are made to NaBrowse project servers, update servers, or
analytics endpoints.

## Data stored locally

NaBrowse does not currently persist browsing history, cookies, localStorage,
or cache to disk. When these features are added, they will be stored locally
on your device only and will not be transmitted to any server.

## Third-party code

NaBrowse uses open-source Rust crates for windowing (winit), GPU rendering
(wgpu), TLS (rustls), async I/O (tokio), and URL parsing (url). These are
compile-time dependencies and do not make independent network connections.

## Changes to this policy

This is a community-driven open-source project. Changes to this privacy policy
will be proposed and discussed publicly in the project repository.

## Contact

For privacy questions, open an issue in the project repository.
