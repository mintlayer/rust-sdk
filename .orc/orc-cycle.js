// ORC review cycle — fan out four reviewer lanes over the ORC diff, collect
// raw findings. Verification and fixes happen in the parent session.
const cwd = '/home/erubboli/Work/nullPointerEnjoyer/rust-sdk';

const brief = `
REVIEW SCOPE: branch feat/indexer-v2-keyset-pagination, i.e. git diff f18563d..HEAD (commits 13e4beb and fb4b93e) in ${cwd}
(run: git show 13e4beb --stat; git show 13e4beb -- <file> to read the diff;
also read full files as needed).

The commit adapts the Mintlayer Rust SDK (crate mintlayer-sdk) to the
api-server v2 changes of mintlayer-core PR #2130. The upstream wire contract
it must satisfy (verified against mintlayer-core @ 2684078c546,
api-server/web-server/src/api/{v2.rs,cursor.rs,error.rs}):

- Cursor envelope (paged_response): {"items":[...],"next_cursor":"<b64>"|null}.
  Endpoints that branch on cursor-param presence (GET /pool, GET /transaction)
  return a PLAIN ARRAY when no cursor param is sent; an EMPTY "cursor=" query
  parameter still counts as present and forces the envelope (and means "start
  from beginning"). Holders and order-book endpoints ALWAYS return the envelope.
- GET /statistics/coin/holders and /statistics/token/{id}/holders: items are
  {"address":"<bech32>","amount":{atoms,decimal}} with the coin's or token's
  decimals; ordered balance DESC; unknown token -> 404 {"error":"Token not found"};
  offset params still work when no cursor is sent; items cap 100, items=0 -> 400.
- GET /order/pair/{pair}/book: side=ask|bid REQUIRED (missing/other -> 400
  {"error":"Bad request"}); pair is {base}_{quote}, coin ticker matched
  case-insensitively, token ids exact case-sensitive bech32, preserved as given.
  Items: {"price":{"atoms":"numer/denom","decimal":"<floored toward zero>"},
  "amount":{atoms,decimal}}. Ask ascending price, bid descending. Cursors are
  side-specific (tags book-ask / book-bid): an ask cursor on a bid walk -> 400
  {"error":"Invalid cursor"}. When the per-request cap (10,000 orders) truncates
  the aggregation the response adds "truncated":true AND next_cursor is null —
  the walk cannot be continued. Book computed fresh per request (no snapshot).
- GET /pool: sort=by_height|by_pledge default by_height; a cursor is only valid
  with by_height (cursor + other sort -> 400 {"error":"Bad request"}); with a
  cursor the offset position is silently overridden server-side (items applies).
- GET /transaction: cursor+offset_mode together -> 400 Bad request;
  offset_mode=legacy|absolute (default legacy; invalid -> 400 invalid offset
  mode); offset path treats absolute offset as a global tx index; listing is
  newest block first. transaction/{id} for a pending tx emits JSON null for
  block_id/timestamp/confirmations (listing items for pending likewise null).
- ALL paginated v2 endpoints: offset u64 default 0, items u32 default 10,
  items=0 and items>100 -> 400 {"error":"Invalid number of items"}.
- Error body shape: {"error":"<message>"}; exact messages include
  "Bad request", "Invalid cursor", "Invalid number of items", "Invalid offset
  mode", "Invalid pools sort order", "Token not found", "Transaction not found".

SDK conventions in this repo (pre-existing, must be respected): one client
method per endpoint on indexer::Client; pub(crate) get(path, &[(&str,String)])
query builder; page_query omits zero offset/items; validate_segment restricts
path segments to [A-Za-z0-9_-]; errors via thiserror enum indexer::Error;
NO new dependencies (Pager is hand-rolled on Pin<Box<dyn Future>>).
`;

const outputInstruction = `
CRITICAL OUTPUT RULE: your ENTIRE final message must be exactly one JSON
array: it must start with '[' and end with ']' and contain nothing else —
no headings, no prose sections, no fences. If you find no defects, your
entire message is exactly []. Do NOT write a human-readable review; the
parent parses your message with JSON.parse.
Each finding:
{"severity":"blocking"|"advisory","file":"<path>","claim":"<one line>",
 "detail":"<what is wrong, exact evidence: file:line, wire shape, or repo convention>",
 "fix":"<concrete suggested fix>"}
Report ONLY real defects: contract violations vs the upstream facts above,
broken invariants, logic bugs, doc/test claims that contradict the code,
convention violations. Style nitpicks are NOT findings. If everything is
correct return []. Findings you are not sure about must be marked advisory,
not blocking.
`;

const lanes = [
  {
    key: 'wire-contract',
    task: `${brief}\n${outputInstruction}
DIMENSION: wire-contract fidelity. Verify every new/changed SDK request shape
and response type against the upstream contract above: query parameters and
their omission rules, the cursor= envelope forcing for pools/transactions, the
always-envelope holders/book endpoints, the order-book item shape and pair
handling, Transaction optionality, holders ordering/decimals, statistics docs.
Files: src/indexer/types.rs, src/indexer/statistics.rs, src/indexer/pool.rs,
src/indexer/transaction.rs, src/indexer/order.rs, src/indexer/mod.rs.`,
  },
  {
    key: 'async-pager',
    task: `${brief}\n${outputInstruction}
DIMENSION: async/state-machine correctness of the pagination layer. Scrutinize
src/indexer/pager.rs and every *_pager constructor (statistics.rs, pool.rs,
transaction.rs, order.rs): FnMut closure capture/move semantics, Send bounds,
cursor propagation and take() logic, done/exhaustion conditions, buffered-item
semantics when mixing next()/next_page(), retry-after-error behavior, truncated
book termination, page-size clamping, per-call clones, accidental unbounded
loops on empty pages.`,
  },
  {
    key: 'errors-serde',
    task: `${brief}\n${outputInstruction}
DIMENSION: error mapping and (de)serialization. Scrutinize src/indexer/error.rs
(from_status_body matching, fallback correctness, sanitization interaction),
src/indexer/mod.rs decode() change, and serde derives in src/indexer/types.rs
(Page<T>, Holder, OrderBook* with #[serde(default)] truncated, Amount usage).
Check: a 400 body that is not JSON or has a different message must still fall
back to Error::Http; the typed variants trigger only on exact upstream
messages; no breaking change to existing variant semantics; clamp_items
boundary conditions (0, 1, 100, 101, u32::MAX).`,
  },
  {
    key: 'tests-docs',
    task: `${brief}\n${outputInstruction}
DIMENSION: tests and documentation coherence. Scrutinize the new tests in
tests/indexer.rs (bottom section) and docs in README.md, docs/indexer.md,
examples/indexer-pagination.rs plus their doc comments in src/. Check: each
required scenario is actually pinned (first page/full walk for pools, holders,
global transactions; both book sides; truncated book stops; last page null;
invalid cursor / invalid num items / token-not-found; pending tx null fields;
offset_mode param; page-size clamp), mocks assert the right query params, the
documented claims match the code exactly (no overclaiming), and the example
compiles against the real API surface.`,
  },
];

const results = await runs.all(lanes.map(lane => ({ key: lane.key, agent: 'reviewer', task: lane.task })));

const findings = {};
for (let i = 0; i < lanes.length; i++) {
  const lane = lanes[i];
  const text = results[i] && results[i].output ? String(results[i].output) : '';
  const start = text.indexOf('[');
  const end = text.lastIndexOf(']');
  let parsed = null;
  if (start !== -1 && end > start) {
    try { parsed = JSON.parse(text.slice(start, end + 1)); } catch (error) { parsed = null; }
  }
  findings[lane.key] = parsed === null
    ? [{ severity: 'blocking', file: '(unparsed)', claim: `lane ${lane.key} did not return JSON`, detail: text.slice(0, 2000), fix: 're-run lane' }]
    : parsed;
}
const total = Object.values(findings).reduce((n, list) => n + list.length, 0);
console.log(`ORC total raw findings: ${total}`);
return { findings, total };
