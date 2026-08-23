# Third-Party Notices

See [Dependency Notices for v1.0.0](docs/DEPENDENCY_NOTICES_V1_0.md).

Third-party dependencies retain their original licenses. QuickPLS remains proprietary.

## Corporate Reputation comparison sample

The bundled Corporate Reputation sample is an independent QuickPLS reproduction
based on the public SmartPLS Corporate Reputation teaching example:
https://www.smartpls.com/documentation/sample-projects/corporate-reputation/

QuickPLS includes a product-owner-supplied, prepared 344-case/31-indicator CSV.
Eleven `-99` missing markers were replaced with their indicator means before
bundling; exact source and output hashes are recorded in
`validation/fixtures/corporate_reputation_smartpls_mean_replaced_v1.provenance.json`.
QuickPLS does not bundle a SmartPLS project, logo, interface asset, or result
screenshot. QuickPLS is not affiliated with or endorsed by SmartPLS GmbH.

## Organizational Identification sample

The bundled Organizational Identification Model uses a product-owner-supplied
305-case values-only CSV. The source workbook and supplied result screenshot are
not bundled. No missing-value replacement, recoding, or row removal was
performed; the preparation and comparison checksums are recorded in
`validation/fixtures/organizational_identification_v1.provenance.json`.

## React Flow (`@xyflow/react`)

MIT License

Copyright (c) 2019-2025 webkid GmbH

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
