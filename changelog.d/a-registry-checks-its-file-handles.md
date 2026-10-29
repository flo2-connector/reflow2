### Fixed

- **A server holding many designs raises its open-file limit at start, and warns
  when its designs could still run out of file handles.** On 2026-10-28 flo2.io
  opened all 64 of its designs and the last two failed with "Too many open
  files": the container's soft limit was Docker's default of 1024 (hard 524288),
  and nothing compared it with `--registry-max-open` and the 64 table files each
  store may keep open. A `--registry-root` server now raises its soft
  `RLIMIT_NOFILE` to the hard limit as it starts (on macOS, as far as
  `kern.maxfilesperproc` and `OPEN_MAX` allow) and says so. It then checks
  `--registry-max-open` × 76 files a design (64 table files plus 12 for its
  write-ahead log, manifest, locks and full-text index) plus 32 of its own
  against the limit. When they do not fit it prints one WARNING naming the three
  numbers and what to change, and keeps running: small designs use far fewer
  files than the worst case. **What to do:** nothing where the hard limit is
  high, as in Docker. Where the warning appears, raise the open-file limit to
  the number it names (`ulimit -n`, Docker's `--ulimit nofile=N:N`, or
  `LimitNOFILE=` in a systemd unit), or lower `--registry-max-open` to the
  number it names.

### Added

- **`GET /readyz` reports the files open against the open-file limit.** Its
  answer gains a second line, `open files: 1013 of 1024 allowed (11 left)`, and
  two headers for a monitor to alarm on, `x-reflow2-open-files` and
  `x-reflow2-open-file-limit`. The count is read from `/proc/self/fd` on Linux;
  on other systems it reads `unknown`. The status code and the first line are
  unchanged, so a probe that reads either is unaffected.
