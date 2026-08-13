from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from validation.webview2_offline_containment import (
    EXPECTED_BROWSER_ARGUMENTS,
    EXPECTED_PROXY_ARGUMENT,
    EXPECTED_PROXY_BIND_ADDRESS,
    EXPECTED_PROXY_URL,
    EXPECTED_QUIC_ARGUMENT,
    validate_webview2_offline_containment,
)


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]

REVIEWED_RUST_BOUNDARY = r'''
const WEBVIEW2_OFFLINE_PROXY_BIND_ADDRESS: &str = "127.0.0.1:17846";
const WEBVIEW2_OFFLINE_PROXY_RESPONSE: &[u8] = b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n";
const MAX_WEBVIEW2_PROXY_REQUEST_HEADER_BYTES: usize = 8 * 1024;
fn bind_webview2_offline_rejection_proxy(bind_address: SocketAddr) -> io::Result<TcpListener> {
    if bind_address.ip() != IpAddr::V4(Ipv4Addr::LOCALHOST) { return Err(blocked()); }
    TcpListener::bind(bind_address)
}
fn reject_webview2_proxy_stream(mut stream: TcpStream) -> io::Result<()> {
    let mut request_prefix = [0_u8; MAX_WEBVIEW2_PROXY_REQUEST_HEADER_BYTES];
    let mut received = 0;
    stream.read(&mut request_prefix[received..]);
    stream.write_all(WEBVIEW2_OFFLINE_PROXY_RESPONSE)?;
    stream.shutdown(Shutdown::Both);
    Ok(())
}
fn serve_webview2_offline_rejection_proxy(listener: TcpListener) -> ! {
    loop { let (stream, _) = listener.accept()?; reject_webview2_proxy_stream(stream); }
}
fn start_webview2_offline_rejection_proxy() -> io::Result<()> {
    let bind_address = WEBVIEW2_OFFLINE_PROXY_BIND_ADDRESS.parse()?;
    let listener = bind_webview2_offline_rejection_proxy(bind_address)?;
    spawn(move || serve_webview2_offline_rejection_proxy(listener));
    Ok(())
}
#[derive(Debug, Serialize)]
struct Fixture;
pub fn run() {
    let _proxy = start_webview2_offline_rejection_proxy().unwrap_or_else(|error| panic!("{error}"));
    tauri::Builder::default();
}
'''


def _write_fixture(root: Path, *, windows: list[dict[str, object]] | None = None) -> Path:
    config_path = root / "src-tauri" / "tauri.conf.json"
    config_path.parent.mkdir(parents=True)
    source_path = root / "src-tauri" / "src" / "lib.rs"
    source_path.parent.mkdir(parents=True)
    source_path.write_text(REVIEWED_RUST_BOUNDARY, encoding="utf-8")
    config_path.write_text(
        json.dumps(
            {
                "app": {
                    "windows": windows
                    or [
                        {
                            "title": "QuickPLS",
                            "proxyUrl": EXPECTED_PROXY_URL,
                            "additionalBrowserArgs": " ".join(EXPECTED_BROWSER_ARGUMENTS),
                        }
                    ],
                    "security": {
                        "csp": "default-src 'self'; connect-src ipc: http://ipc.localhost"
                    },
                }
            }
        ),
        encoding="utf-8",
    )
    return config_path


class WebView2OfflineContainmentTests(unittest.TestCase):
    def test_repository_contract_passes(self) -> None:
        report = validate_webview2_offline_containment(REPOSITORY_ROOT)

        self.assertTrue(report["passed"])
        self.assertTrue(report["runtime_verification_required"])
        self.assertEqual(report["additional_browser_arguments"], list(EXPECTED_BROWSER_ARGUMENTS))
        self.assertEqual(report["quic_browser_argument"], EXPECTED_QUIC_ARGUMENT)
        self.assertEqual(report["proxy_browser_argument"], EXPECTED_PROXY_ARGUMENT)
        self.assertEqual(report["proxy_url"], EXPECTED_PROXY_URL)
        self.assertEqual(report["proxy_bind_address"], EXPECTED_PROXY_BIND_ADDRESS)
        self.assertTrue(report["rejects_all_proxy_requests"])
        self.assertTrue(report["fail_closed_before_webviews"])
        self.assertEqual(report["renderer_connect_sources"], ["ipc:", "http://ipc.localhost"])

    def test_rejects_each_missing_containment_argument(self) -> None:
        for missing in EXPECTED_BROWSER_ARGUMENTS:
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                remaining = [argument for argument in EXPECTED_BROWSER_ARGUMENTS if argument != missing]
                _write_fixture(
                    root,
                    windows=[{
                        "title": "QuickPLS",
                        "proxyUrl": EXPECTED_PROXY_URL,
                        "additionalBrowserArgs": " ".join(remaining),
                    }],
                )
                with self.assertRaises(SystemExit):
                    validate_webview2_offline_containment(root)

    def test_rejects_unreviewed_argument_or_endpoint_exemption(self) -> None:
        extras = (
            "--remote-debugging-port=9222",
            "--proxy-bypass-list=*.example.test",
            "--host-resolver-rules=MAP example.test 127.0.0.1",
        )
        for extra in extras:
            with self.subTest(extra=extra), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                _write_fixture(
                    root,
                    windows=[
                        {
                            "title": "QuickPLS",
                            "proxyUrl": EXPECTED_PROXY_URL,
                            "additionalBrowserArgs": f"{' '.join(EXPECTED_BROWSER_ARGUMENTS)} {extra}",
                        }
                    ],
                )
                with self.assertRaises(SystemExit):
                    validate_webview2_offline_containment(root)

    def test_rejects_missing_drifted_or_reordered_proxy_browser_argument(self) -> None:
        reviewed = list(EXPECTED_BROWSER_ARGUMENTS)
        mutations = {
            "missing": reviewed[:-1],
            "drifted port": [
                *reviewed[:-1],
                "--proxy-server=http://127.0.0.1:17847",
            ],
            "remote proxy": [
                *reviewed[:-1],
                "--proxy-server=http://10.0.0.10:17846",
            ],
            "wrong order": [
                *reviewed[:-2],
                EXPECTED_PROXY_ARGUMENT,
                reviewed[-2],
            ],
        }
        for name, arguments in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                _write_fixture(
                    root,
                    windows=[{
                        "title": "QuickPLS",
                        "proxyUrl": EXPECTED_PROXY_URL,
                        "additionalBrowserArgs": " ".join(arguments),
                    }],
                )

                with self.assertRaises(SystemExit):
                    validate_webview2_offline_containment(root)

    def test_rejects_missing_drifted_or_reordered_quic_browser_argument(self) -> None:
        reviewed = list(EXPECTED_BROWSER_ARGUMENTS)
        quic_index = reviewed.index(EXPECTED_QUIC_ARGUMENT)
        mutations = {
            "missing": reviewed[:quic_index] + reviewed[quic_index + 1 :],
            "drifted": [
                *reviewed[:quic_index],
                "--disable-quic=false",
                *reviewed[quic_index + 1 :],
            ],
            "wrong order": [
                *reviewed[:quic_index],
                reviewed[quic_index + 1],
                EXPECTED_QUIC_ARGUMENT,
                *reviewed[quic_index + 2 :],
            ],
        }
        for name, arguments in mutations.items():
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                _write_fixture(
                    root,
                    windows=[{
                        "title": "QuickPLS",
                        "proxyUrl": EXPECTED_PROXY_URL,
                        "additionalBrowserArgs": " ".join(arguments),
                    }],
                )

                with self.assertRaises(SystemExit):
                    validate_webview2_offline_containment(root)

    def test_rejects_any_uncontained_window(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            _write_fixture(
                root,
                windows=[
                    {
                        "title": "QuickPLS",
                        "proxyUrl": EXPECTED_PROXY_URL,
                        "additionalBrowserArgs": " ".join(EXPECTED_BROWSER_ARGUMENTS),
                    },
                    {"title": "Uncontained auxiliary window"},
                ],
            )
            with self.assertRaises(SystemExit):
                validate_webview2_offline_containment(root)

    def test_rejects_missing_or_changed_loopback_proxy_url(self) -> None:
        for proxy_url in (None, "http://127.0.0.1:17847", "http://10.0.0.10:17846"):
            with self.subTest(proxy_url=proxy_url), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                config_path = _write_fixture(root)
                config = json.loads(config_path.read_text(encoding="utf-8"))
                if proxy_url is None:
                    del config["app"]["windows"][0]["proxyUrl"]
                else:
                    config["app"]["windows"][0]["proxyUrl"] = proxy_url
                config_path.write_text(json.dumps(config), encoding="utf-8")

                with self.assertRaises(SystemExit):
                    validate_webview2_offline_containment(root)

    def test_rejects_remote_or_drifted_rust_bind_address(self) -> None:
        for bind_address in ("0.0.0.0:17846", "127.0.0.1:17847"):
            with self.subTest(bind_address=bind_address), tempfile.TemporaryDirectory() as temp:
                root = Path(temp)
                _write_fixture(root)
                source_path = root / "src-tauri" / "src" / "lib.rs"
                source = source_path.read_text(encoding="utf-8").replace(
                    EXPECTED_PROXY_BIND_ADDRESS,
                    bind_address,
                )
                source_path.write_text(source, encoding="utf-8")

                with self.assertRaises(SystemExit):
                    validate_webview2_offline_containment(root)

    def test_rejects_forwarding_logic_in_rejection_handler(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            _write_fixture(root)
            source_path = root / "src-tauri" / "src" / "lib.rs"
            source = source_path.read_text(encoding="utf-8").replace(
                "stream.write_all(WEBVIEW2_OFFLINE_PROXY_RESPONSE)?;",
                "let _upstream = TcpStream::connect(\"198.51.100.1:443\")?;\n"
                "    stream.write_all(WEBVIEW2_OFFLINE_PROXY_RESPONSE)?;",
            )
            source_path.write_text(source, encoding="utf-8")

            with self.assertRaises(SystemExit):
                validate_webview2_offline_containment(root)

    def test_rejects_proxy_start_after_tauri_builder(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            _write_fixture(root)
            source_path = root / "src-tauri" / "src" / "lib.rs"
            source = source_path.read_text(encoding="utf-8").replace(
                "let _proxy = start_webview2_offline_rejection_proxy().unwrap_or_else(|error| panic!(\"{error}\"));\n"
                "    tauri::Builder::default();",
                "tauri::Builder::default();\n"
                "    let _proxy = start_webview2_offline_rejection_proxy().unwrap_or_else(|error| panic!(\"{error}\"));",
            )
            source_path.write_text(source, encoding="utf-8")

            with self.assertRaises(SystemExit):
                validate_webview2_offline_containment(root)

    def test_rejects_remote_renderer_connection_source(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            config_path = _write_fixture(root)
            config = json.loads(config_path.read_text(encoding="utf-8"))
            config["app"]["security"]["csp"] += " https://telemetry.example.test"
            config_path.write_text(json.dumps(config), encoding="utf-8")

            with self.assertRaises(SystemExit):
                validate_webview2_offline_containment(root)

    def test_rejects_duplicate_json_keys(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            config_path = root / "src-tauri" / "tauri.conf.json"
            config_path.parent.mkdir(parents=True)
            config_path.write_text('{"app": {}, "app": {}}', encoding="utf-8")

            with self.assertRaises(SystemExit):
                validate_webview2_offline_containment(root)


if __name__ == "__main__":
    unittest.main()
