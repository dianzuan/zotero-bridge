# Bootstrap Lifecycle Review

Date: 2026-05-08

This note records the code-quality review for the targeted bootstrap bugfix work.
It is intentionally scoped to the add-on bootstrap path and the RPC-driven
reload loop that development workers use while iterating on Zotron.

## Reviewed surface

- `addon/bootstrap.js` owns Zotero bootstrap-extension registration, default
  preference loading, the compiled `content/scripts/zotron.js` script load, and
  shutdown cleanup.
- `src/index.ts` attaches the compiled hook surface to `Zotero.Zotron`.
- `src/hooks.ts` registers the `/zotron/rpc` endpoint, initializes preference
  defaults/migrations, registers the preference pane, and unregisters the
  endpoint during shutdown.
- `src/handlers/system.ts` exposes `system.reload`, which invalidates Gecko's
  startup cache and asks Zotero's add-on manager to reload Zotron after the RPC
  response has had time to flush.

## Runtime invariants

Keep these invariants intact when modifying the bootstrap path:

1. `startup()` must register chrome and load `zotron.js` before invoking
   `Zotero.Zotron.hooks.onStartup()`.
2. `Zotero.Zotron.data.rootURI` must be populated before hook startup so the
   preference pane can resolve `content/preferences.xhtml` and
   `content/preferences.js`.
3. `shutdown()` must call `hooks.onShutdown()` before destructing the chrome
   handle so the HTTP endpoint is removed while the loaded hook code is still
   available.
4. Non-application shutdowns must clear `chromeHandle`; application shutdown may
   return early because Zotero is already tearing down process-wide services.
5. `system.reload` must return `{ "status": "reloading" }` immediately and do
   the actual reload asynchronously; callers should not wait for the add-on to
   disappear while the response is still in flight.
6. The reload path should invalidate `startupcache` before `addon.reload()` so
   development builds re-read the updated bundled script instead of stale Gecko
   cache entries.

## Code-quality observations

- The lifecycle split is small and maintainable: bootstrap concerns stay in
  `addon/bootstrap.js`, while endpoint and preference-pane registration stay in
  TypeScript hooks.
- The shutdown sequence is correctly defensive (`Zotero.Zotron?.hooks`) and safe
  for partial startup failures.
- Preference default and migration logic already has focused unit coverage in
  `test/hooks.test.ts`; reload behavior is covered in
  `test/handlers/system.test.ts`.
- The main future regression risk is accidental reordering in `startup()` or
  making `system.reload` synchronous. Either change can break hot reload or leave
  workers with a dropped HTTP response.

## Manual smoke checklist

After any bootstrap or reload change, run the normal automated checks and then
perform this Zotero smoke path when a desktop Zotero instance is available:

```bash
npm run build
curl -s -X POST http://localhost:23119/zotron/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"system.ping","id":1}'
curl -s -X POST http://localhost:23119/zotron/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"system.reload","id":2}'
sleep 2
curl -s -X POST http://localhost:23119/zotron/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"system.ping","id":3}'
```

Expected result: the first and final `system.ping` calls succeed, and
`system.reload` returns `{"status":"reloading"}` without hanging.
