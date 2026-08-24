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

## NumPy hypergeometric sampler

QuickPLS's count-space hypergeometric sampler is adapted from the HRUA and
direct-complement implementation in NumPy's
`numpy/random/src/distributions/random_hypergeometric.c`, with additional
large-integer numerical safeguards for QuickPLS's admitted frequency envelope.
The adapted source is pinned to NumPy commit
`ffa72d99810dc54fa4222c3ffc623c4b268191b1`; that source file's SHA-256 is
`1fc39d0b5aea55bc7a2b7da75268018c6554af81cbf763528ea89030f76b106f`.

BSD 3-Clause License

Copyright (c) 2005-2017, NumPy Developers.
All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

* Redistributions of source code must retain the above copyright notice, this
  list of conditions and the following disclaimer.
* Redistributions in binary form must reproduce the above copyright notice,
  this list of conditions and the following disclaimer in the documentation
  and/or other materials provided with the distribution.
* Neither the name of the NumPy Developers nor the names of any contributors
  may be used to endorse or promote products derived from this software without
  specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
