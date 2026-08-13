# WebView2 Offline Runtime Boundary

QuickPLS is designed so its analytical workflows do not require remote network
access, accounts, or cloud services. The QuickPLS application and page make no
external requests. The packaged Windows shell applies a reviewed set of WebView2 browser
arguments to suppress background services that could otherwise create remote
connections independently of QuickPLS application code. Those switches and the
product-owned rejection proxy contain ordinary WebView requests, but they are
not a kernel network boundary for the Microsoft-managed WebView2 runtime.

The current pinned WebView2 runtime has been observed opening browser-owned
Microsoft TLS connections even when the exact proxy, background-network,
component-update, sync, metrics-recording and QUIC switches are present. Those
connections were not initiated by QuickPLS page or IPC requests, but they still
invalidate a literal zero-process-egress claim. QuickPLS must therefore keep
strict zero-egress acceptance red unless an OS-enforced boundary proves it.

## Frozen product configuration

Every window in `src-tauri/tauri.conf.json` must use exactly:

```text
--disable-background-networking
--disable-component-update
--disable-sync
--metrics-recording-only
--disable-quic
--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection
--proxy-server=http://127.0.0.1:17846
```

The first four arguments suppress Chromium background requests, component
downloads, profile synchronization, and metrics upload. `--disable-quic`
prevents WebView2 from using QUIC over UDP outside the HTTP proxy boundary. The
final argument routes ordinary WebView HTTP(S) traffic to the product-owned
rejection proxy.
The disabled-feature token immediately before it is not an added QuickPLS
policy: setting `additionalBrowserArgs` replaces Wry's default argument string,
so Wry's three defaults must be carried forward verbatim.

QuickPLS currently resolves Wry 0.55.1. In that Windows implementation,
`additional_browser_args` is selected with `unwrap_or_else`; translation of
Wry's `proxy_config` into `--proxy-server=...` occurs only inside the fallback.
Because QuickPLS supplies an explicit argument string, `proxyUrl` alone does not
reach WebView2. The exact `--proxy-server=http://127.0.0.1:17846` switch is
therefore frozen in `additionalBrowserArgs` as well as the declarative
`proxyUrl`. Removing either representation is a contract failure.

Microsoft documents that `AdditionalBrowserArguments` is passed to the WebView2
browser process, that repeated switches are not generally merged, and that
environment or registry values may append or override arguments. Chromium
documents the selected switches in its source:

- [Microsoft WebView2 environment options](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2environmentoptions)
- [Chromium browser switches](https://chromium.googlesource.com/chromium/src.git/+/master/chrome/common/chrome_switches.cc)
- [Chromium metrics switches](https://chromium.googlesource.com/chromium/src.git/+/master/components/metrics/metrics_switches.cc)

The renderer CSP separately restricts `connect-src` to Tauri IPC:
`ipc:` and `http://ipc.localhost`. The packaged application does not hard-code a
remote debugging port, hostname mapping, remote endpoint allowlist, or network
exemption.

## Fail-closed rejection proxy

Every configured Tauri window uses this exact proxy URL:

```text
http://127.0.0.1:17846
```

Before `tauri::Builder` can create a configured WebView, the Rust desktop entry
point binds `127.0.0.1:17846` and starts the rejection service. Startup panics
with an explicit offline-boundary error if the port cannot be bound; QuickPLS
does not fall back to a direct connection. This fixed-port ownership also means
a second QuickPLS process cannot start while the first process owns the proxy
port.

The listener accepts only IPv4 loopback connections. It drains at most 8 KiB of
request headers so Windows can close the connection cleanly, but it does not
decode or use the requested destination, resolve a hostname, connect to an
upstream socket, or copy traffic. It immediately returns `HTTP/1.1 403
Forbidden` with `Connection: close` for ordinary HTTP and HTTPS `CONNECT`
requests, then closes the socket. A failed listener or rejected request
therefore fails closed for ordinary proxied web content. It does not prove that
Microsoft-managed WebView2 browser services cannot use a separate control-plane
or diagnostic transport.

## Localhost boundary

The offline policy does not block process-local transports. Tauri production
assets and IPC use the local application origin (`tauri.localhost` and
`ipc.localhost`); these internal origins are handled by the application runtime
and remain constrained by the CSP. Packaged acceptance may append a loopback CDP port through
`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`; this is test instrumentation and must
not be present in the shipped Tauri configuration.

## Verification and limitations

Run the fail-closed static contract without launching the application:

```powershell
python validation/webview2_offline_containment.py
python -m unittest validation.test_webview2_offline_containment
```

This contract rejects missing, changed, reordered, or additional browser
arguments—including `--disable-quic` and the explicit proxy switch—a missing or
changed proxy URL, a non-loopback or drifted Rust bind address, forwarding logic
in the rejection handler, proxy startup after Tauri WebView creation, an
uncontained new window, duplicate JSON keys, and any renderer `connect-src`
expansion. Focused Rust tests send both HTTP and HTTPS `CONNECT` requests to an
ephemeral instance of the same handler, assert the exact rejection, and verify
that the requested upstream sentinel received no connection.

The source contract does not by itself prove packaged runtime behavior. A
freshly rebuilt desktop executable must still pass the packaged process-tree
monitor with zero non-loopback TCP connections before any zero-egress claim.
The current candidate does not pass that gate. WebView2 arguments can be
changed by machine policy or the `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`
environment variable, and future WebView2 versions can change proxy behavior;
those remain runtime gate inputs, not static assumptions.

QuickPLS may accurately say that its analytical workflows require no internet
connection, account, or cloud service, and that the QuickPLS application/page
makes no external requests. The installer and bundled runtime path may also be
described as working without an internet connection after their dedicated
offline tests pass. It must not make a literal "fully offline", "no telemetry",
or "zero egress" process-tree claim. Strict zero-egress mode requires a
separate, independently tested OS-enforced fixed-WebView2 containment gate,
such as administrator-installed Windows Firewall/WFP rules covering both the
QuickPLS executable and an app-private fixed WebView2 runtime, including IPv4,
IPv6, TCP and UDP. That stricter architecture and gate remain pending.

The embedded offline WebView2 installer removes a network prerequisite for
installation. The WebView2 runtime remains a Microsoft-managed component after
installation; its separate updater service is outside the QuickPLS process tree
and outside this application-level boundary.
