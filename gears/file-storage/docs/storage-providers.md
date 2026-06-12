# Object Storage Providers — Feature Timeline

Comparative timeline of features relevant to the FileStorage module across the three major managed providers and three S3-compatible self-hosted backends. Cells show **the year a feature became GA** (or the current value, when relevant). Where a feature is absent, the cell explains why.

| Feature | AWS S3 | Google Cloud Storage | MS Azure Blob Storage | Virtuozzo (VHI) | Ceph (RGW) | MinIO | s3s-fs |
|---|---|---|---|---|---|---|---|
| **Service GA** | 2006 | 2010 (preview) / 2011 (GA) | Feb 2010 | Virtuozzo Storage in production **2014**; S3 object surface ~**2016** | **2006** (open-source); RGW S3 surface in **2012** (Argonaut/Bobtail) | **2015-06-17** (open-source release; founded Nov 2014) | **2023-01-17** (first crate publish; experimental S3 server on local FS) |
| **Customers** (publicly stated) | "millions of customers" (Mar 2026, AWS official) | not publicly disclosed for Cloud Storage; Google Cloud overall has hundreds of thousands of paying customers | "millions of customers" across Azure Storage (Microsoft Ignite 2025) | enterprise / MSP focus; commercial customers in service-provider market (no public count) | self-hosted, no central count; backbone of OpenStack and IBM Storage Ceph; production at CERN, telcos, large gov agencies | self-hosted, no central count; tens of thousands of orgs report production use | N/A — sample/test S3 server (used as the local-disk recipe in FileStorage) |
| **Total data stored** | **"hundreds of exabytes"** (Mar 2026, AWS official); 500 trillion objects, ~200 M req/s | not publicly disclosed; individual Colossus filesystems exceed 10 EB; Apple alone holds ≥ 8 EB on GCP | **"exabytes"** of capacity; ≥ 100 EB read/written per month, 1 quadrillion transactions/month (2023 figures) | not centrally reported; multi-PB per cluster typical | aggregated multi-EB across deployments (CERN: hundreds of PB single cluster); not centrally reported | N/A — depends on deployment | N/A — bounded by host filesystem |
| **Max object size** (current) | **48.8 TiB** (2025) — was 5 TB from 2010, raised Dec 2025 | **5 TiB** (steady since ~2014; was 100 GB → 1 TB earlier) | **190.7 TiB** / 200 TB (preview Mar 2021, GA ~2022) — was 4.75 TiB from May 2016, 200 GB before | mirrors S3 multipart limits (5 TiB) | mirrors S3 multipart limits (5 TiB via 10 000 × 5 GiB parts; tunable) | mirrors S3 multipart limits (5 TB historically; tracks AWS) | bounded by underlying filesystem (e.g. ext4: 16 TiB; XFS: 8 EiB) |
| **Max object name (key)** | 1024 bytes UTF-8 (2006) | 1024 bytes UTF-8 (2010) | 1024 chars (2010) | 1024 bytes UTF-8 (2016, S3-compat) | 1024 bytes UTF-8 (2012, S3-compat) | 1024 bytes UTF-8 (2015, S3-compat) | 1024 bytes UTF-8 (2023, S3-compat) |
| **Max user metadata** | **2 KB** total (2006) | **8 KiB** per value (2010) | **8 KiB** total per blob (2010) | 2 KB total (2016, S3-compat) | 2 KB total (2012, S3-compat) | 2 KB total (2015, S3-compat) | 2 KB (2023, S3-compat) |
| **Range read** (`Range: bytes=N-M` on GET) | ✅ 2006 (HTTP Range, single & multi-range) | ✅ 2010 — `Range` header (or `?alt=media` + Range) | ✅ 2010 — `Range` / `x-ms-range` | ✅ 2016 (S3-compat) | ✅ 2012 (S3-compat) | ✅ 2015 (S3-compat) | ✅ 2023 (S3-compat) |
| **File + metadata atomic on PUT** | ✅ 2006 | ✅ 2010 | ✅ 2010 | ✅ 2016 | ✅ 2012 | ✅ 2015 | ✅ 2023 |
| **Server-minted upload session ID** | ✅ `UploadId` — multipart upload, **2010** | ✅ `sessionURI` — resumable upload, 2010 | ⚠️ Block-IDs are **client-generated** (2010); no equivalent of server-minted UploadId | ✅ `UploadId` (2016, S3-compat multipart) | ✅ `UploadId` (2012, S3-compat multipart) | ✅ `UploadId` (2015, S3-compat multipart) | ✅ `UploadId` (2023, S3-compat multipart) |
| **Versioning support** (per-bucket) | **2010-03-16** (GA, beta from Feb 8) | ~**2011** (since GA of Cloud Storage) | **2020-09-07** (GA) | ✅ supported (per VHI Storage User Guide) | **2015** (Hammer release, RGW S3 versioning API) | **2020-06-13** (PR #9377) | ✅ implemented (2023+, S3-compat) |
| **VersionId format** | opaque string ≤ 1024 bytes (2010) | int64 monotonic `generation` (2010) | timestamp `2020-07-03T08:13:22.6694804Z` (2020) | opaque string (S3-compat) | opaque string (2015, S3-compat) | UUID v4 (2020) | opaque string (2023, S3-compat) |
| **Read-after-write consistency** (strong) | **2020-12-01** — before that, eventual consistency for overwrites/deletes | ✅ **2010** (since launch) | ✅ **2010** (since launch) | ✅ **2014** — Virtuozzo Storage strongly consistent by design ("like Ceph, only faster") | ✅ **2006** — Ceph RADOS is strongly consistent by design | ✅ 2015 (strict consistency by design) | ✅ 2023 (single-process, FS-backed) |
| **Optimistic concurrency on read** (`If-Match` on GET) | ✅ 2006 | ✅ 2010 | ✅ 2010 | ✅ 2016 (S3-compat) | ✅ 2012 (S3-compat) | ✅ 2015 (S3-compat) | ✅ 2023 (S3-compat) |
| **Optimistic concurrency on write** (`If-Match` on PUT) | **2024-11-25** — `If-Match` by ETag added | ✅ **2010** — via `x-goog-if-generation-match` and HTTP `If-Match` | ✅ **2010** — via `If-Match` ETag | ⚠️ S3 surface — supported per S3-compat parity, no public confirmation of full AWS-2024 alignment | ✅ supported on RGW for years; refinements in **Tentacle (Nov 2025)** for conditional Delete/MultiDelete/Put/MultiWrite | ✅ **~2023** — MinIO shipped conditional writes ahead of AWS; aligned with S3 contract in `RELEASE.2024-09-13` | ✅ 2023 (per crate feature list) |
| **Write-if-not-exists** (`If-None-Match: *` on PUT) | **2024-08** | ✅ **2010** — `x-goog-if-generation-match: 0` | ✅ **2010** — `If-None-Match: *` | ⚠️ S3-compat parity (depends on VHI release; not separately announced) | ✅ ~Tentacle 2025 (S3-compat parity) | ✅ ~2023 | ✅ 2023 |
| **CAS by ETag on upload** | **2024-11-25** | ✅ **2010** | ✅ **2010** | ⚠️ S3-compat parity | ✅ Tentacle 2025 (S3-compat parity) | ✅ ~2023 | ✅ 2023 |
| **ETag bound to content** (1 content → 1 ETag) | ✅ 2006 — MD5 of bytes (multipart: hex-N format, deterministic) | ⚠️ ETag is opaque; docs recommend `generation` instead | ❌ ETag is an **opaque write-token**; rotates on every write incl. metadata-only changes | ✅ 2016 — MD5 (mirrors S3 contract) | ✅ 2012 — MD5 (mirrors S3 contract) | ✅ 2015 — MD5 (mirrors S3 contract) | ✅ 2023 — MD5 (mirrors S3) |
| **CAS by content version** (ABA-safe upload precondition) | ❌ **not available** — `If-Match` uses ETag (MD5), with ABA window on bit-identical re-upload; VersionId only on read (`?versionId=`) | ✅ **2010** — `x-goog-if-generation-match`; generation rotates on every PUT regardless of byte equality | ⚠️ **indirect** — ETag rotates on every write; `If-Match` is functionally equivalent (no separate write-token CAS) | ❌ no native generation/metageneration; same ABA window as AWS S3 | ❌ no native generation/metageneration; same ABA window as AWS S3 | ❌ no native generation/metageneration; same ABA window as AWS S3 | ❌ inherits S3 contract |
| **CAS by metadata version** (separate from content) | ❌ no concept of metadata version | ✅ **2010** — `x-goog-if-metageneration-match` (independent counter) | ❌ ETag is shared with content; cannot CAS on metadata-only | ❌ no metageneration in S3-compat surface | ❌ no metageneration in S3-compat surface | ❌ no metageneration in S3-compat surface | ❌ no metageneration |
| **"Upload if VersionId=N"** (slide scenario #1) | ❌ impossible | ✅ `x-goog-if-generation-match: N` | ⚠️ via `If-Match: <etag>` (functional equivalent) | ❌ impossible | ❌ impossible | ❌ impossible | ❌ impossible |
| **"Upload if meta was not changed"** (slide scenario #2) | ❌ impossible | ✅ `x-goog-if-metageneration-match: N` | ❌ impossible (ETag covers both) | ❌ impossible | ❌ impossible | ❌ impossible | ❌ impossible |

## Reading the table

- **AWS S3** has the weakest conditional-write contract in the trio of managed providers — only since November 2024 does it have `If-Match` on PUT, and it still lacks both VersionId-CAS and metadata-CAS. Strong RAW consistency arrived only in December 2020.
- **GCS** has the richest CAS surface — two independent counters (`generation`, `metageneration`) yield ABA-safe content CAS and metadata-only CAS, both since launch. ETag exists but Google recommends generation/metageneration.
- **Azure Blob** has had `If-Match` ETag CAS since 2010, but its ETag conflates content and metadata changes — there is no separate metadata-version token. Versioning (per-blob VersionId) arrived only in September 2020.
- **Virtuozzo (VHI)** is a commercial software-defined storage and compute platform targeting service providers and enterprises. Virtuozzo Storage entered production in 2014 ("like Ceph, only faster"); the S3 object surface, Object Lock, bucket policies, and versioning are all available. It inherits the S3-compat conditional-write contract — same ABA caveats as AWS/Ceph/MinIO.
- **Ceph (RGW)** is the oldest open-source player (2006); RADOS is strongly consistent from day one. The RGW S3 gateway arrived in 2012, versioning in Hammer (2015), and the Tentacle release (Nov 2025) shipped fixes/refinements for conditional writes. Ceph powers OpenStack, IBM Storage Ceph, CERN, and many large telco/gov deployments.
- **MinIO** tracks the S3 contract closely; notably shipped conditional writes (`If-Match` on PUT) ahead of AWS. Versioning was added in June 2020. As an S3-compat surface it inherits S3's lack of generation/metageneration, so the same ABA caveats apply.
- **s3s-fs** is a sample/test S3 server backed by the local filesystem — used as the local-disk recipe in the FileStorage module. It implements the S3 surface (versioning, multipart, conditional GET/PUT, ETag) but is not intended as a production backend; its capacity, durability, and consistency are bounded by the host filesystem.

## Implications for the FileStorage module

- The `s3-compatible` adapter targets the lowest-common-denominator surface (AWS until 2024). Self-healing via ETag (ADR-0004) and the per-backend `versioning` flag for ABA-safe strong CAS (ADR-0005) compensate for the gaps that GCS and Azure close natively.
- A future `gcs` or `azure-blob` adapter could exploit native preconditions for stronger guarantees (e.g. metageneration-CAS for `PUT /meta`), but per `cpt-cf-file-storage-principle-modular-backend-roster`, the SDK contract must remain uniform — backend-specific stronger guarantees become an implementation detail, not an API surface.
- The `48.8 TiB` parameter on the AWS reference slide is recent (Dec 2025); deployments running on older S3 contracts still cap at 5 TB.

## Condensed Summary

Same rows as above, but each cell shows only the year a feature became available (or current value where year doesn't apply). ✅ marks the leader in the row (earliest year, or largest value); ❌ marks "not available". Empty cells = approximation / N/A.

| Feature | AWS S3 | GCS | Azure | Virtuozzo | Ceph (RGW) | MinIO | s3s-fs |
|---|---|---|---|---|---|---|---|
| Service GA | ✅ 2006 | 2010 | 2010 | 2014 (S3: 2016) | ✅ 2006 (RGW: 2012) | 2015 | 2023 |
| Customers | ✅ "millions" | — | "millions" | — | — | — | — |
| Total data stored | ✅ ~hundreds of EB | tens of EB | exabytes | multi-PB per cluster | multi-EB aggregate | — | — |
| Max object size (year of current limit) | 2025 (48.8 TiB) | 2014 (5 TiB) | ✅ 2021 (190.7 TiB) | 2016 (5 TiB) | 2012 (5 TiB) | 2015 | 2023 |
| Max object name (1024 B) | ✅ 2006 | 2010 | 2010 | 2016 | 2012 | 2015 | 2023 |
| Max user metadata | 2006 (2 KB) | ✅ 2010 (8 KiB) | ✅ 2010 (8 KiB) | 2016 (2 KB) | 2012 (2 KB) | 2015 (2 KB) | 2023 (2 KB) |
| Range read | ✅ 2006 | 2010 | 2010 | 2016 | 2012 | 2015 | 2023 |
| File + metadata atomic on PUT | ✅ 2006 | 2010 | 2010 | 2016 | 2012 | 2015 | 2023 |
| Server-minted upload session ID | ✅ 2010 | 2010 | ❌ | 2016 | 2012 | 2015 | 2023 |
| Versioning support | ✅ 2010 | 2011 | 2020 | 2016 | 2015 | 2020 | 2023 |
| Read-after-write consistency (strong) | 2020 | 2010 | 2010 | 2014 | ✅ 2006 | 2015 | 2023 |
| Optimistic concurrency on read (If-Match GET) | ✅ 2006 | 2010 | 2010 | 2016 | 2012 | 2015 | 2023 |
| Optimistic concurrency on write (If-Match PUT) | 2024 | ✅ 2010 | ✅ 2010 | 2016 | 2025 | 2023 | 2023 |
| Write-if-not-exists (If-None-Match: *) | 2024 | ✅ 2010 | ✅ 2010 | 2016 | 2025 | 2023 | 2023 |
| CAS by ETag on upload | 2024 | ✅ 2010 | ✅ 2010 | 2016 | 2025 | 2023 | 2023 |
| ETag bound to content | ✅ 2006 | — (opaque) | ❌ | 2016 | 2012 | 2015 | 2023 |
| CAS by content version (ABA-safe) | ❌ | ✅ 2010 | — (indirect via ETag) | ❌ | ❌ | ❌ | ❌ |
| CAS by metadata version | ❌ | ✅ 2010 | ❌ | ❌ | ❌ | ❌ | ❌ |
| Upload if VersionId=N | ❌ | ✅ 2010 | — (indirect via ETag) | ❌ | ❌ | ❌ | ❌ |
| Upload if meta was not changed | ❌ | ✅ 2010 | ❌ | ❌ | ❌ | ❌ | ❌ |

**Score per provider** (count of ✅ vs ❌):

| Provider | ✅ leader rows | ❌ missing rows |
|---|---|---|
| AWS S3 | 9 | 4 |
| GCS | 11 | 1 |
| Azure | 5 | 4 |
| Virtuozzo | 0 | 5 |
| Ceph (RGW) | 2 | 5 |
| MinIO | 0 | 5 |
| s3s-fs | 0 | 5 |

**GCS** is the broadest leader by far — 11 leader rows, only one ❌ (no public customer-count claim). **AWS** leads in raw scale (size, customers, total data, age) but loses on conditional-write semantics. **Azure** is solid in foundational features but lacks server-minted upload IDs and has no metadata-version CAS. **Ceph (RGW)** ties AWS for oldest service GA (both 2006) and for strong RAW consistency (Ceph had it from day one, AWS only since 2020). **Virtuozzo, MinIO, s3s-fs** are pure S3-compat surfaces — they inherit AWS's conditional-write gaps.

## Sources

- [AWS S3 turns 20 — "hundreds of exabytes", 500 trillion objects, millions of customers (Mar 2026, The Register)](https://www.theregister.com/2026/03/16/aws_s3_turns_20/)
- [How Amazon S3 Stores 350 Trillion Objects with 11 Nines of Durability (ByteByteGo)](https://blog.bytebytego.com/p/how-amazon-s3-stores-350-trillion)
- [Azure Storage — 1 quadrillion transactions/month, 100 EB/month read+write (Reflecting on 2023)](https://azure.microsoft.com/en-us/blog/reflecting-on-2023-azure-storage/)
- [Azure Storage innovations — exabyte-scale capacity, AI workloads](https://azure.microsoft.com/en-us/blog/azure-storage-innovations-unlocking-the-future-of-data/)
- [Apple is Google's largest cloud customer — 8+ EB stored on GCP (Datacenter Dynamics)](https://www.datacenterdynamics.com/en/news/report-apple-is-googles-largest-cloud-customer-for-storage/)
- [How Colossus optimizes data placement (10+ EB filesystems)](https://cloud.google.com/blog/products/storage-data-transfer/how-colossus-optimizes-data-placement-for-performance)
- [Amazon S3 — Strong Read-After-Write Consistency (Dec 2020)](https://aws.amazon.com/blogs/aws/amazon-s3-update-strong-read-after-write-consistency/)
- [Amazon S3 — Multipart Upload (Nov 2010)](https://aws.amazon.com/blogs/aws/amazon-s3-multipart-upload/)
- [Amazon S3 Versioning — GA (Mar 16, 2010)](https://aws.amazon.com/blogs/aws/amazon-s3-versioning-now-ready/)
- [Amazon S3 — Conditional writes (Aug 2024)](https://aws.amazon.com/about-aws/whats-new/2024/08/amazon-s3-conditional-writes/)
- [Amazon S3 — `If-Match` conditional writes (Nov 25, 2024)](https://aws.amazon.com/about-aws/whats-new/2024/11/amazon-s3-functionality-conditional-writes/)
- [Amazon S3 — Maximum object size 50 TB (Dec 2, 2025)](https://aws.amazon.com/about-aws/whats-new/2025/12/amazon-s3-maximum-object-size-50-tb/)
- [Amazon S3 — Multipart upload limits (current 48.8 TiB)](https://docs.aws.amazon.com/AmazonS3/latest/userguide/qfacts.html)
- [Azure — Blob versioning GA (Sep 7, 2020)](https://azure.microsoft.com/en-us/updates?id=azure-blob-versioning-is-now-general-available)
- [Azure — Manage concurrency in Blob Storage](https://learn.microsoft.com/en-us/azure/storage/blobs/concurrency-manage)
- [Azure — Larger Block Blobs GA (4.75 TiB, May 2016)](https://azure.microsoft.com/en-us/blog/general-availability-larger-block-blobs-in-azure-storage/)
- [Azure — 200 TB block blobs preview (Mar 2021)](https://azure.microsoft.com/en-us/blog/run-high-scale-workloads-on-blob-storage-with-new-200-tb-object-sizes/)
- [Azure — Blob Storage scalability targets (current 190.7 TiB)](https://learn.microsoft.com/en-us/azure/storage/blobs/scalability-targets)
- [Azure — Setting and retrieving metadata (8 KB total)](https://learn.microsoft.com/en-us/rest/api/storageservices/setting-and-retrieving-properties-and-metadata-for-blob-resources)
- [GCS — Request preconditions (generation/metageneration)](https://cloud.google.com/storage/docs/request-preconditions)
- [GCS — Object Versioning](https://cloud.google.com/storage/docs/object-versioning)
- [GCS — Object metadata (8 KiB per value)](https://cloud.google.com/storage/docs/metadata)
- [GCS — Quotas & limits (5 TiB max object size)](https://cloud.google.com/storage/quotas)
- [MinIO — Five Years in the Making (history, founded Nov 2014)](https://blog.min.io/five-years-in-the-making/)
- [MinIO — first public release (Wikipedia, Jun 17, 2015)](https://en.wikipedia.org/wiki/MinIO)
- [MinIO — Bucket Versioning (PR #9377, merged Jun 2020)](https://github.com/minio/minio/pull/9377)
- [MinIO — Conditional Write Feature (blog post, ahead of AWS)](https://blog.min.io/leading-the-way-minios-conditional-write-feature-for-modern-data-workloads/)
- [s3s-fs on crates.io (first published 2023-01-17)](https://crates.io/crates/s3s-fs)
- [s3s — S3 API surface (versioning, multipart, conditional GET/PUT)](https://github.com/Nugine/s3s)
- [Ceph — official site](https://ceph.io/)
- [Ceph (software) — Wikipedia (founded 2004, open-source 2006)](https://en.wikipedia.org/wiki/Ceph_(software))
- [Ceph Hammer release notes (RGW S3 versioning, 2015)](https://docs.ceph.com/en/latest/releases/hammer/)
- [Ceph Tentacle 20.2.0 release notes (Nov 2025, conditional write fixes)](https://ceph.io/en/news/blog/2025/v20-2-0-tentacle-released/)
- [Ceph Object Gateway S3 API documentation](https://docs.ceph.com/en/latest/radosgw/s3/)
- [Virtuozzo — official site](https://www.virtuozzo.com/)
- [Virtuozzo — High-Performance S3 Storage Solutions for Data Lakes](https://www.virtuozzo.com/company/blog/high-performance-s3-storage-for-data-lakes/)
- [Virtuozzo Hybrid Infrastructure 7.0 — External Storage, 3x Faster S3, Secure-by-Default Compute (Jul 2025)](https://www.virtuozzo.com/company/blog/product-updates/external-storage-secure-policy-s3-hybrid-infrastructure-7-0/)
- [Virtuozzo Storage is "like Ceph, only faster" (Blocks and Files, Feb 2019)](https://blocksandfiles.com/2019/02/07/virtuozzo-storage-is-like-ceph-only-faster/)
- [Virtuozzo Hybrid Infrastructure 7.1 Storage User Guide (Sep 2025)](https://docs.virtuozzo.com/pdf/virtuozzo_hybrid_infrastructure_7_1_users_guide.pdf)
