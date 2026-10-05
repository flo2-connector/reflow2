# Updating reflow2 without losing your design

Short answer: **updating reflow2 does not touch your design.** The binary and the design are
separate things in every deployment shape, and this page says exactly where your design lives so
you can be sure of that rather than hopeful about it.

It also says plainly what reflow2 does **not** do for you — backups — and what it gives you to
build them with.

---

## Where your design actually lives

| | What it is | Survives an update? |
|---|---|---|
| `.reflow2/graph/` | a RocksDB store — the working copy | yes, it is never inside the binary |
| `.reflow2/graph.id.json` | identity: **which design this is** | yes — and see the warning below |
| `.reflow2/graph.meta.json` | the version stamp of the reflow2 that wrote it | yes |
| `docs/design/<name>.json` | **the committed export — the durable record** | yes, it is in your repo |
| `reflow2-content/` | the content-addressed blob store | yes |

**`.reflow2/` is gitignored, and that is deliberate.** The store is a *cache*. The **export** is
the record: a complete, deterministic, content-hashed document that lives in your repo and moves
with it. If the store is ever lost, `--import` rebuilds it from the export and you lose nothing
that was committed.

---

## Two things can be out of date, and they update separately

This is the single most common confusion, so it is stated before the procedures:

| What | How it updates | What it is |
|---|---|---|
| **reflow2 itself** — the binary and the kit on this machine | re-run the installer (below) | needs the network |
| **a project already set up** — its `AGENTS.md`, slash commands, MCP config, hooks | **`reflow2 update`** | purely local, never downloads |

**Updating reflow2 does not update a project you set up earlier.** The per-project files were
copied when you ran `reflow2 init`, and they stay at that generation until you say otherwise. A
project installed at 0.16.0 under a 0.30.0 binary gets *current instructions driving an old kit*,
and nothing announces it.

```bash
cd my-project
reflow2 update --check     # what would change, writes nothing
reflow2 update             # bring this project forward
```

It reuses the harness you chose the first time, keeps files you edited, and **never touches your
design graph**. It refuses a project that was never set up rather than quietly performing a first
install — absence of a kit is not staleness, and the message names `reflow2 init` instead.

`reflow2 update` also *reports* when the binary itself is behind. It will not update it for you:
that needs the network and is the procedure below.

## Updating a locally-installed reflow2

### Before you install: export and commit each design, back up `.reflow2/`

An upgrade is one-way, and its rollback point has to be taken **before** the new binary is on the
machine: the first open by a new reflow2 can repair the store, so an export or a backup taken after
the install is a post-upgrade one. For each design:

```bash
reflow2-mcp write export_graph '{"path":"docs/design/<name>/"}'
git add docs/design && git commit -m "Design record before the reflow2 upgrade"
cp -a .reflow2 ../<name>.reflow2.before-upgrade     # with no server holding it
```

### Install

```bash
curl -fsSL https://raw.githubusercontent.com/flo2-connector/reflow2/main/tools/install.sh | sh
```

The installer **keeps the binary it replaces** as `reflow2-mcp.<its version>` beside the new one,
named from what that binary's own `--version` says, and prints the path. Only the last one is kept.
To roll back: `cp ~/.local/bin/reflow2-mcp.<old version> ~/.local/bin/reflow2-mcp`, then restart
your agent session.

Then **restart your agent session** — an MCP server is a running process, and a reconnect does not
replace one that is already running. Only a full restart picks up a new binary.

**What happens the first time the new binary opens your store:** it reads the version stamp beside
it and tells you what it found.

- *"this graph was written by reflow2 0.23.0 … you are running 0.24.0. **Additive only — everything
  in it still reads.**"* — schema growth only ever adds, so an older store is safe.
- *"this graph carried no version stamp; recording reflow2 0.24.0 from now on"* — it predates the
  check. Nothing is wrong.

Every door says it, the one-shot ones included (`reflow2 read`, `--call`, `--export`, `--import`,
`--diff`): one line on stderr, only when there is an upgrade, a downgrade or a repair to report.
The stamp, `.reflow2/graph.meta.json`, keeps `previous_reflow2_version` and the last repair an open
made (`last_repair_on_open`), so an upgrade can be confirmed after the fact. `reflow2 read` changes
nothing in the design; it does refresh this stamp, as it always has, because the stamp is the
store's own bookkeeping.

If a release needs a migration step, it ships an `upgrading-to-v0.X.0.md` alongside it. Those are
the exception, not the rule.

> ⚠️ **Downgrading is warned, not refused.** Opening a store with an *older* reflow2 than the one
> that last wrote it says *"THIS GRAPH WAS LAST WRITTEN BY reflow2 X; you are running Y, which is
> BEHIND it"*, on every door, and the server also serves `served_by.behind_record`. Everything still
> reads, but every write the older binary makes can put its own schema defaults onto the newer
> record. A one-shot door leaves the newer stamp in place; a server rewrites it to its own version
> and records the newer one as `previous_reflow2_version`. To roll back cleanly, restore the
> `.reflow2/` backup taken before the upgrade, or re-`--import` the export you committed then.

### With only the binary (no installer)

reflow2 works with only `reflow2-mcp` on the machine: the verbs are `reflow2-mcp read …` and
`reflow2-mcp write …`. **First check whether `reflow2` is the installer's wrapper:**

```bash
head -2 ~/.local/bin/reflow2
```

If the second line says `# reflow2 — installed by reflow2_install.py`, this machine has the
installer's wrapper: **update by re-running the installer** above, never by hand. Otherwise, to
update by hand: take the backup above, keep the old binary
(`cp ~/.local/bin/reflow2-mcp ~/.local/bin/reflow2-mcp.$(~/.local/bin/reflow2-mcp --version | awk '{print $NF}')`),
then unpack the release's `reflow2-mcp-<platform>.tar.gz` over `~/.local/bin/reflow2-mcp`. Do not
`ln -s reflow2-mcp reflow2` over an existing wrapper. If `reflow2` is already a symlink to the
binary, the installer replaces the link with its wrapper and says so.

---

## Updating a containerised reflow2

**Replacing the image does not touch your design, because none of it is in the image.** Stop the
old container, start the new one against the same volume:

```bash
docker pull ghcr.io/flo2-connector/reflow2/reflow2-mcp:<version>
docker stop reflow2 && docker rm reflow2
docker run -d --name reflow2 -p 8080:8080 -v /srv/reflow2-data:/data \
  ghcr.io/flo2-connector/reflow2/reflow2-mcp:<version>
```

The image is `ghcr.io/<owner>/<repo>/reflow2-mcp` — the shorter `ghcr.io/<owner>/reflow2-mcp`
404s, which reads exactly like a missing image. From the first release after v0.76.0 it is one
index for `linux/amd64` and `linux/arm64`, and **the release notes carry the index digest**: pin
`ghcr.io/flo2-connector/reflow2/reflow2-mcp@sha256:<digest>` wherever a tag moving under you
would matter. A tag can be re-pushed; a digest names one image forever.

**The image moved with the repository, at v0.79.0.** From v0.79.0 on it is published as
`ghcr.io/flo2-connector/reflow2/reflow2-mcp`. Every image up to and including v0.78.0 stays where
it was published, at `ghcr.io/sligara7/reflow2/reflow2-mcp`, and is not copied to the new name. So
a script, compose file or CI job that pins the old name keeps pulling the old releases and never
sees a new one: change the name when you move to v0.79.0 or later.

This is tested rather than asserted: stopping a container and starting a new one against the same
volume leaves `graph_id` byte-identical, with no re-mint warning — the new container adopts the
existing design instead of opening an empty one beside it.

### ⚠️ The one mistake that looks like data loss

**Mount the directory that CONTAINS the store, never the store itself.**

```
/data/graphs/myproject/graph            ← the store
/data/graphs/myproject/graph.id.json    ← identity  ┐  these are SIBLINGS of the store,
/data/graphs/myproject/graph.meta.json  ← version   ┘  not inside it
```

Mounting `.../graph` leaves the sidecars behind, and the design can no longer be named. reflow2
will not guess which design the store holds, so it does not open it, and your data stays untouched
on disk. What you will see:

- **one design** (`--graph-path`): the server comes up serving a single tool,
  `reflow2_unavailable`, and its handshake and log name the missing `graph.id.json` and say to
  mount the parent;
- **a registry** (`--registry-root`): the store is listed as **found and not served**, at
  startup, on a GET of `/`, and in the refusal for an unknown `/g/<id>/`, with the same remedy.
  The root is re-read on every request, so the design is served again as soon as its identity is
  back, with no restart.

Mount the parent.

### A store opened through a symlink, or as `--graph-path .`

Up to v0.79.0 the identity file was written beside the path **as typed**. A store first opened
through a symlink kept `<link-name>.id.json` beside the LINK. A store opened as `--graph-path .`
kept `..id.json` INSIDE itself. In both cases, opening it later by its real path was refused as
"lost its identity file". Releases after v0.79.0 always write it beside the store's **real**
directory, however the path was typed.

They still read those older places, so every store that opened before still opens. When reflow2
finds the identity only in an older place, it writes a copy beside the store and says so once, on
stderr and in `loop_status`. It never moves or deletes the old file.

If a store is refused as having lost its identity file:

1. **Put the design's id file beside the store**, at `<store>.id.json` (for example
   `.reflow2/graph.id.json`).
2. If the store was ever opened through a symlink, the file is beside that link, as
   `<link-name>.id.json`. `find ~ -name '*.id.json'` finds it; copy it beside the store.
3. If there is no copy anywhere, the refusal prints the design id the store's own data carries and
   an id file that names it. Write that file beside the store.

A refused open writes nothing, so trying again after each step is safe.

**Use a real block device or local volume, not NFS.** RocksDB's exclusive lock is a filesystem
lock, and network filesystems honour those unreliably. A lock that silently fails to exclude is
how two processes end up writing one store.

---

## Backups are yours, not reflow2's

**reflow2 does not back your design up, and will not.** That is a deliberate boundary, not a gap.

Backup is a property of *where your data lives* — your volume, your storage account, your
retention window, your compliance rules. reflow2 knows none of that, and a design tool that
invented an answer would be wrong for most people who installed it.

**What reflow2 gives you to build one with:**

- **`export_graph` is a complete snapshot.** One deterministic, content-hashed document containing
  every node and edge. Two exports of an unchanged design are byte-identical, so it diffs cleanly
  and a corrupted copy is detectable.
- **`--import` is the restore.** It rebuilds a working store from an export, and preserves
  `graph_id`, so the restored design *is* the same design rather than a copy that shares a name.
- **For the repo-file model, git is already your off-host backup.** The export is committed, so
  every clone and every remote is a copy, versioned and timestamped, for free.

**A hosted deployment is the case that needs real work**, because a server has no repo. A
reasonable shape, borrowed from production practice:

1. Snapshot before every deploy — `export_graph` to a file on the volume.
2. Push that file off-host, and **verify it landed** rather than assuming it did.
3. After the deploy, compare node counts by type against the pre-deploy snapshot; treat any
   *decrease* as an alarm and print the restore command.
4. Keep receipts, so an audit can correlate a deploy with what the design looked like either side.

Steps 1 and 2 are the ones that matter. Step 3 is cheap and catches the failure that silence would
otherwise hide.

---

## Quick checks

```bash
# Which design is this, and which reflow2 wrote it?
cat .reflow2/graph.id.json .reflow2/graph.meta.json

# Does the committed record still match the build? (also checks your dependency pins)
python3 tools/reflow2_check.py --export docs/design/<name>.json

# Rebuild a working store from the committed record
reflow2-mcp --graph-path .reflow2/graph --import docs/design/<name>.json
```

If `reflow2_check` passes, the design in your repo describes the build you have. That is the
property worth protecting across an update, and it is the one this page exists to keep true.
