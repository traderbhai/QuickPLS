"""Fail-closed static contract for the packaged WebView2 network boundary."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
EXPECTED_PROXY_BIND_ADDRESS = "127.0.0.1:17846"
EXPECTED_PROXY_URL = f"http://{EXPECTED_PROXY_BIND_ADDRESS}"
EXPECTED_PROXY_ARGUMENT = f"--proxy-server={EXPECTED_PROXY_URL}"
EXPECTED_QUIC_ARGUMENT = "--disable-quic"

# Setting additionalBrowserArgs replaces Wry's default string. Keep its three
# disabled features, QUIC shutdown, and proxy flag in the reviewed replacement.
EXPECTED_BROWSER_ARGUMENTS = (
    "--disable-background-networking",
    "--disable-component-update",
    "--disable-sync",
    "--metrics-recording-only",
    EXPECTED_QUIC_ARGUMENT,
    "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection",
    EXPECTED_PROXY_ARGUMENT,
)
EXPECTED_CONNECT_SOURCES = ("ipc:", "http://ipc.localhost")


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def _read_json(path: Path) -> dict[str, Any]:
    def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f"duplicate key {key!r}")
            result[key] = value
        return result

    def reject_non_finite(value: str) -> None:
        raise ValueError(f"non-finite numeric constant {value!r}")

    try:
        document = json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=reject_duplicate_keys,
            parse_constant=reject_non_finite,
        )
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError) as error:
        raise SystemExit(f"Cannot read strict Tauri configuration at {path}: {error}") from error
    _require(isinstance(document, dict), "Tauri configuration must contain a JSON object")
    return document


def _connect_sources(csp: str) -> tuple[str, ...]:
    directives: dict[str, tuple[str, ...]] = {}
    for raw_directive in csp.split(";"):
        parts = raw_directive.strip().split()
        if not parts:
            continue
        name, *sources = parts
        _require(name not in directives, f"CSP contains duplicate {name!r} directive")
        directives[name] = tuple(sources)
    _require("connect-src" in directives, "Tauri CSP must define an explicit connect-src directive")
    return directives["connect-src"]


def _read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise SystemExit(f"Cannot read UTF-8 source at {path}: {error}") from error


def _source_between(source: str, start: str, end: str) -> str:
    start_index = source.find(start)
    _require(start_index >= 0, f"Rust offline boundary is missing {start!r}")
    end_index = source.find(end, start_index + len(start))
    _require(end_index >= 0, f"Rust offline boundary is missing successor {end!r}")
    return source[start_index:end_index]


def _validate_rust_rejection_proxy(source_path: Path) -> None:
    source = _read_text(source_path)
    _require(
        f'const WEBVIEW2_OFFLINE_PROXY_BIND_ADDRESS: &str = "{EXPECTED_PROXY_BIND_ADDRESS}";'
        in source,
        "Rust WebView2 offline proxy bind address must exactly match the configured loopback proxy",
    )
    _require(
        'const WEBVIEW2_OFFLINE_PROXY_RESPONSE: &[u8] = b"HTTP/1.1 403 Forbidden\\r\\n'
        in source,
        "Rust WebView2 offline proxy must define the reviewed unconditional 403 response",
    )
    _require(
        "const MAX_WEBVIEW2_PROXY_REQUEST_HEADER_BYTES: usize = 8 * 1024;" in source,
        "Rust WebView2 offline proxy must retain the reviewed bounded request-header drain",
    )

    bind_source = _source_between(
        source,
        "fn bind_webview2_offline_rejection_proxy",
        "fn reject_webview2_proxy_stream",
    )
    loopback_guard = "if bind_address.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST)"
    listener_bind = "TcpListener::bind(bind_address)"
    _require(loopback_guard in bind_source, "Rust proxy bind must reject every non-127.0.0.1 address")
    _require(listener_bind in bind_source, "Rust proxy must bind its reviewed SocketAddr directly")
    _require(
        bind_source.index(loopback_guard) < bind_source.index(listener_bind),
        "Rust proxy loopback guard must run before TcpListener::bind",
    )

    reject_source = _source_between(
        source,
        "fn reject_webview2_proxy_stream",
        "fn serve_webview2_offline_rejection_proxy",
    )
    _require(
        "stream.write_all(WEBVIEW2_OFFLINE_PROXY_RESPONSE)?;" in reject_source,
        "Rust proxy must unconditionally write the reviewed rejection response",
    )
    _require(
        "stream.shutdown(Shutdown::Both)" in reject_source,
        "Rust proxy must close rejected connections",
    )
    _require(
        "let mut request_prefix = [0_u8; MAX_WEBVIEW2_PROXY_REQUEST_HEADER_BYTES];"
        in reject_source
        and "stream.read(&mut request_prefix[received..])" in reject_source,
        "Rust proxy must only drain a bounded request-header prefix before rejecting",
    )
    for forbidden in (
        ".read_to_",
        "TcpStream::connect",
        "ToSocketAddrs",
        "from_utf8",
        "String::from",
        "Command::new",
        "reqwest",
        "ureq",
    ):
        _require(
            forbidden not in reject_source,
            f"Rust rejection handler must not inspect or forward requests ({forbidden!r})",
        )

    serve_source = _source_between(
        source,
        "fn serve_webview2_offline_rejection_proxy",
        "fn start_webview2_offline_rejection_proxy",
    )
    _require("listener.accept()" in serve_source, "Rust proxy service must accept local connections")
    _require(
        "reject_webview2_proxy_stream(stream)" in serve_source,
        "Rust proxy service must send every accepted connection to the rejection handler",
    )

    start_source = _source_between(
        source,
        "fn start_webview2_offline_rejection_proxy",
        "#[derive(Debug, Serialize)]",
    )
    for required in (
        "WEBVIEW2_OFFLINE_PROXY_BIND_ADDRESS",
        "bind_webview2_offline_rejection_proxy(bind_address)?",
        "serve_webview2_offline_rejection_proxy(listener)",
    ):
        _require(required in start_source, f"Rust proxy startup is missing {required!r}")

    run_index = source.find("pub fn run()")
    _require(run_index >= 0, "Rust desktop entry point pub fn run() is missing")
    start_call_index = source.find("start_webview2_offline_rejection_proxy().unwrap_or_else", run_index)
    builder_index = source.find("tauri::Builder::default()", run_index)
    _require(start_call_index >= 0, "Desktop entry point must fail closed when proxy startup fails")
    _require(builder_index >= 0, "Desktop entry point must create the Tauri builder")
    _require(
        start_call_index < builder_index,
        "WebView2 offline proxy must bind before Tauri creates configured webviews",
    )


def validate_webview2_offline_containment(root: Path = ROOT) -> dict[str, Any]:
    """Validate every configured window and the renderer connection boundary."""

    root = root.resolve()
    config_path = root / "src-tauri" / "tauri.conf.json"
    source_path = root / "src-tauri" / "src" / "lib.rs"
    config = _read_json(config_path)

    app = config.get("app")
    _require(isinstance(app, dict), "Tauri configuration must define app")
    windows = app.get("windows")
    _require(isinstance(windows, list) and windows, "Tauri app.windows must be a non-empty list")

    validated_windows: list[str] = []
    for index, window in enumerate(windows):
        _require(isinstance(window, dict), f"Tauri app.windows[{index}] must be an object")
        label = window.get("label") or window.get("title") or f"window[{index}]"
        arguments = window.get("additionalBrowserArgs")
        _require(
            isinstance(arguments, str) and arguments.strip(),
            f"{label}: additionalBrowserArgs must define the offline WebView2 boundary",
        )
        tokens = tuple(arguments.split())
        _require(
            tokens == EXPECTED_BROWSER_ARGUMENTS,
            f"{label}: additionalBrowserArgs must exactly match the reviewed offline containment arguments",
        )
        _require(len(tokens) == len(set(tokens)), f"{label}: duplicate WebView2 browser argument")
        proxy_url = window.get("proxyUrl")
        _require(
            proxy_url == EXPECTED_PROXY_URL,
            f"{label}: proxyUrl must route all WebView network requests to the reviewed loopback rejection proxy",
        )
        validated_windows.append(str(label))

    security = app.get("security")
    _require(isinstance(security, dict), "Tauri app.security configuration is required")
    csp = security.get("csp")
    _require(isinstance(csp, str) and csp.strip(), "Tauri app.security.csp must be a non-empty string")
    connect_sources = _connect_sources(csp)
    _require(
        connect_sources == EXPECTED_CONNECT_SOURCES,
        "Tauri CSP connect-src must allow only ipc: and http://ipc.localhost",
    )
    _validate_rust_rejection_proxy(source_path)

    return {
        "schema_version": 2,
        "target": "QuickPLS packaged WebView2 offline containment",
        "passed": True,
        "config": config_path.relative_to(root).as_posix(),
        "windows": validated_windows,
        "additional_browser_arguments": list(EXPECTED_BROWSER_ARGUMENTS),
        "quic_browser_argument": EXPECTED_QUIC_ARGUMENT,
        "proxy_browser_argument": EXPECTED_PROXY_ARGUMENT,
        "proxy_url": EXPECTED_PROXY_URL,
        "proxy_bind_address": EXPECTED_PROXY_BIND_ADDRESS,
        "proxy_implementation": source_path.relative_to(root).as_posix(),
        "rejects_all_proxy_requests": True,
        "fail_closed_before_webviews": True,
        "renderer_connect_sources": list(EXPECTED_CONNECT_SOURCES),
        "runtime_verification_required": True,
    }


def main() -> None:
    print(json.dumps(validate_webview2_offline_containment(), indent=2))


if __name__ == "__main__":
    main()
